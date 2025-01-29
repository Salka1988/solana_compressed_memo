#[cfg(test)]
mod tests {
    use anyhow::anyhow;
    use extanded_spl::instruction::ExtendedSPLMemoInstruction;
    use extanded_spl::processor::process_instruction;
    use extanded_spl::processor::CompressedMemo;
    use light_hasher::{DataHasher, Poseidon};
    use solana_program::instruction::InstructionError;
    use solana_program_test::*;
    use solana_sdk::transaction::TransactionError;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use std::fs;
    use std::path::Path;
    use std::process;
    use std::process::Child;
    use std::process::{Command, Stdio};
    use std::sync::Arc;
    use std::sync::Mutex;

    #[tokio::test]
    async fn test_create_compressed_memo_success() {
        let program_id = Pubkey::new_unique();

        let test = ProgramTest::new("extended_spl", program_id, processor!(process_instruction));

        let (mut banks_client, payer, recent_blockhash) = test.start().await;
        let new_account = Keypair::new();

        let memo_str = "Hello from LightHasher!";
        let ix_data = ExtendedSPLMemoInstruction::CreateCompressedMemo {
            memo: memo_str.to_string(),
        };

        let instruction = Instruction::new_with_borsh(
            program_id,
            &ix_data,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(new_account.pubkey(), true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
            ],
        );

        let mut tx = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        tx.sign(&[&payer, &new_account], recent_blockhash);

        banks_client.process_transaction(tx).await.unwrap();

        let acct_data = banks_client
            .get_account(new_account.pubkey())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(acct_data.data.len(), 32);

        let expected_hash = CompressedMemo {
            memo: memo_str.to_string(),
        }
        .hash::<Poseidon>();

        assert_eq!(acct_data.data[..32], expected_hash.expect("BD"));
    }

    #[tokio::test]
    async fn test_create_compressed_memo_max_length() {
        let program_id = Pubkey::new_unique();

        let test = ProgramTest::new("extanded_spl", program_id, processor!(process_instruction));

        let (mut banks_client, payer, recent_blockhash) = test.start().await;
        let new_account = Keypair::new();

        let memo_str = "a".repeat(128);
        let ix_data = ExtendedSPLMemoInstruction::CreateCompressedMemo {
            memo: memo_str.to_string(),
        };

        let instruction = Instruction::new_with_borsh(
            program_id,
            &ix_data,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(new_account.pubkey(), true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
            ],
        );

        let mut tx = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        tx.sign(&[&payer, &new_account], recent_blockhash);

        banks_client.process_transaction(tx).await.unwrap();

        let acct_data = banks_client
            .get_account(new_account.pubkey())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(acct_data.data.len(), 32);

        let expected_hash = CompressedMemo { memo: memo_str }.hash::<Poseidon>();
        assert_eq!(acct_data.data[..32], expected_hash.expect("BD"));
    }

    #[tokio::test]
    async fn test_create_compressed_memo_exceed_max_length() {
        let program_id = Pubkey::new_unique();

        let test = ProgramTest::new("extanded_spl", program_id, processor!(process_instruction));

        let (mut banks_client, payer, recent_blockhash) = test.start().await;
        let new_account = Keypair::new();

        let memo_str = "a".repeat(129);
        let ix_data = ExtendedSPLMemoInstruction::CreateCompressedMemo { memo: memo_str };

        let instruction = Instruction::new_with_borsh(
            program_id,
            &ix_data,
            vec![
                AccountMeta::new(payer.pubkey(), true),
                AccountMeta::new(new_account.pubkey(), true),
                AccountMeta::new_readonly(solana_program::system_program::ID, false),
            ],
        );

        let mut tx = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        tx.sign(&[&payer, &new_account], recent_blockhash);

        let result = banks_client.process_transaction(tx).await;

        assert!(result.is_err());
        let err = result.unwrap_err().unwrap();

        assert_eq!(
            err,
            TransactionError::InstructionError(0, InstructionError::Custom(1))
        );
    }

    fn build_bpf_program(project_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(project_dir).exists() {
            return Err(format!("Project directory '{}' does not exist", project_dir).into());
        }

        println!("Building the BPF program in {}...", project_dir);
        if !Command::new("cargo")
            .args([
                "build-bpf",
                "--manifest-path",
                &format!("{}/Cargo.toml", project_dir),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success()
        {
            return Err("Failed to build BPF program.".into());
        }

        println!("BPF build done at: {}", project_dir);
        Ok(())
    }

    fn deploy_program(project_dir: &str) -> Result<String, Box<dyn std::error::Error>> {
        if !Path::new(project_dir).exists() {
            return Err(format!("Project directory '{}' does not exist", project_dir).into());
        }

        println!("Deploying the program...");
        let keypair_path = "~/.config/solana/id.json";

        if !Path::new(keypair_path).exists() {
            println!("Creating default keypair...");
            if !Command::new("solana-keygen")
                .args(["new", "--no-passphrase", "-o", keypair_path])
                .status()?
                .success()
            {
                return Err("Failed to create default keypair.".into());
            }
        }

        let pubkey = String::from_utf8_lossy(
            &Command::new("solana")
                .args(["address", "-k", keypair_path])
                .output()?
                .stdout,
        )
        .trim()
        .to_string();

        Command::new("solana")
            .args(["airdrop", "10", &pubkey, "--url", "http://127.0.0.1:8899"])
            .status()?;

        let so_path = format!("{}/target/deploy/extanded_spl.so", project_dir);
        if !Path::new(&so_path).exists() {
            return Err("Program file does not exist. Build it first.".into());
        }

        let deploy_output = Command::new("solana")
            .args([
                "program",
                "deploy",
                "--keypair",
                keypair_path,
                &so_path,
                "--url",
                "http://127.0.0.1:8899",
            ])
            .output()?;

        let output_str = String::from_utf8_lossy(&deploy_output.stdout);
        let program_id = output_str
            .lines()
            .find(|line| line.starts_with("Program Id:"))
            .and_then(|line| line.split("Program Id:").nth(1))
            .map(str::trim)
            .map(String::from)
            .ok_or("Failed to parse program ID.")?;

        println!("Program deployed successfully: {}", program_id);
        Ok(program_id)
    }

    fn run_typescript_test_locally(
        project_dir: &str,
        program_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(project_dir).exists() {
            return Err(format!("Project directory '{}' does not exist", project_dir).into());
        }

        Command::new("npm")
            .args(["install"])
            .current_dir(project_dir)
            .status()?;

        Command::new("npx")
            .args([
                "mocha",
                "-r",
                "ts-node/register",
                "tests/ts/test_compressed_memo.ts",
                "--",
                &format!("--program-id={}", program_id),
            ])
            .current_dir(project_dir)
            .status()?;

        println!("TypeScript test completed.");
        Ok(())
    }

    pub struct TestValidator {
        ledger_dir: Arc<Mutex<String>>,
        child: Option<Child>,
    }

    impl TestValidator {
        pub fn new() -> Self {
            let ledger_dir = Arc::new(Mutex::new(String::from("test-ledger")));
            let ledger_dir_clone = Arc::clone(&ledger_dir);

            ctrlc::set_handler(move || {
                println!("Interrupt signal received, cleaning up...");
                if let Ok(dir) = ledger_dir_clone.lock() {
                    TestValidator::cleanup_dir(&dir);
                }
                process::exit(0);
            })
            .expect("Error setting Ctrl+C handler");

            Self {
                ledger_dir,
                child: None,
            }
        }

        fn cleanup_dir(dir: &str) {
            if Path::new(dir).exists() {
                if let Err(e) = fs::remove_dir_all(dir) {
                    eprintln!("Failed to remove ledger directory '{}': {}", dir, e);
                } else {
                    println!("Cleaned up ledger directory: {}", dir);
                }
            }
        }

        fn start_test_validator(&mut self) -> Result<(), anyhow::Error> {
            let unique_dir = {
                let mut dir = self.ledger_dir.lock().map_err(|e| anyhow!(e.to_string()))?;
                let new_dir = format!("test-ledger-{}", rand::random::<u32>());
                *dir = new_dir.clone();
                new_dir
            };

            println!("Created test directory: {}", unique_dir);

            for (file, desc) in [
                ("validator-identity.json", "Validator Identity"),
                ("validator-vote-account.json", "Validator Vote Account"),
                ("validator-stake-account.json", "Validator Stake Account"),
                ("faucet-keypair.json", "Faucet Keypair"),
            ] {
                let path = format!("{}/{}", unique_dir, file);
                println!("Generating keypair: {}", path);
                if !Command::new("solana-keygen")
                    .args(["new", "--no-passphrase", "-so", &path])
                    .status()
                    .map_err(|e| anyhow!("Failed to run solana-keygen: {}", e))?
                    .success()
                {
                    return Err(anyhow!("Failed to generate {} keypair.", desc));
                }
            }

            println!("Creating genesis ledger...");
            if !Command::new("solana-genesis")
                .args([
                    "--hashes-per-tick",
                    "sleep",
                    "--faucet-lamports",
                    "500000000000000000",
                    "--bootstrap-validator",
                    &format!("{}/validator-identity.json", unique_dir),
                    &format!("{}/validator-vote-account.json", unique_dir),
                    &format!("{}/validator-stake-account.json", unique_dir),
                    "--faucet-pubkey",
                    &format!("{}/faucet-keypair.json", unique_dir),
                    "--ledger",
                    &unique_dir,
                    "--cluster-type",
                    "development",
                ])
                .status()
                .map_err(|e| anyhow!("Failed to run solana-genesis: {}", e))?
                .success()
            {
                return Err(anyhow!("Failed to create genesis ledger."));
            }

            println!("Starting Solana Test Validator...");
            let child = Command::new("solana-test-validator")
                .args(["--reset", "--ledger", &unique_dir])
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .map_err(|e| anyhow!("Failed to start solana-test-validator: {}", e))?;

            // this should be some primitive backoff mechanism
            for attempt in 1..=10 {
                if let Ok(output) = Command::new("solana")
                    .args(["cluster-version", "--url", "http://127.0.0.1:8899"])
                    .stderr(Stdio::piped()) // Capture stderr
                    .output()
                {
                    if output.status.success() {
                        println!("Solana Test Validator is ready.");
                        break;
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!(
                            "Attempt {}: Solana Test Validator is not ready. Error: {}",
                            attempt, stderr
                        );
                    }
                } else {
                    eprintln!("Attempt {}: Failed to execute readiness check.", attempt);
                }

                if attempt == 10 {
                    eprintln!("Terminating Solana Test Validator due to readiness failure.");
                    return Err(anyhow!(
                        "Solana Test Validator did not become ready after 10 attempts."
                    ));
                }

                std::thread::sleep(std::time::Duration::from_secs(2));
            }

            println!(
                "Solana Test Validator is running with ledger: {}",
                unique_dir
            );

            self.child = Some(child);

            Ok(())
        }

        pub async fn spawn_validator_thread(&mut self) -> anyhow::Result<()> {
            self.start_test_validator()?;
            Ok(())
        }
    }

    impl Drop for TestValidator {
        fn drop(&mut self) {
            if let Some(mut child) = self.child.take() {
                println!("Killing Solana Test Validator process...");
                if let Err(e) = child.kill() {
                    eprintln!("Failed to kill Solana Test Validator process: {}", e);
                }
                if let Err(e) = child.wait() {
                    eprintln!("Failed to wait for Solana Test Validator process: {}", e);
                }
            }

            if let Ok(dir) = self.ledger_dir.lock() {
                TestValidator::cleanup_dir(&dir);
            }
        }
    }

    #[tokio::test]
    async fn test_solana_program_fully_in_docker_via_commands(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut validator = TestValidator::new();
        validator.spawn_validator_thread().await?;

        let project_dir = std::env::current_dir()?.to_string_lossy().to_string();
        build_bpf_program(&project_dir)?;

        let program_id = deploy_program(&project_dir)?;
        run_typescript_test_locally(&project_dir, &program_id)?;

        // Cleanup happens automatically when `validator` goes out of scope
        Ok(())
    }
}
