#[cfg(test)]
mod tests {
    use solana_program_test::*;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use light_hasher::{DataHasher, Poseidon};
    use solana_program::instruction::InstructionError;
    use solana_sdk::transaction::TransactionError;
    use extanded_spl::instruction::ExtendedSPLMemoInstruction;
    use extanded_spl::processor::process_instruction;
    use extanded_spl::processor::CompressedMemo;
    use std::path::Path;
    use std::fs;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::process;
    use std::{process::{Command, Stdio}, thread};

    #[tokio::test]
    async fn test_create_compressed_memo_success() {
        let program_id = Pubkey::new_unique();

        let mut test = ProgramTest::new(
            "extended_spl",
            program_id,
            processor!(process_instruction),
        );

        let (mut banks_client, payer, recent_blockhash) = test.start().await;
        let new_account = Keypair::new();

        let memo_str = "Hello from LightHasher!";
        let ix_data = ExtendedSPLMemoInstruction::CreateCompressedMemo {
            memo: memo_str.to_string(),
        };

        let instruction = Instruction::new_with_borsh(program_id, &ix_data,vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(new_account.pubkey(), true),
            AccountMeta::new_readonly(solana_program::system_program::ID, false), // Add the system program
        ]);

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

        let mut test = ProgramTest::new(
            "extanded_spl",
            program_id,
            processor!(process_instruction),
        );

        let (mut banks_client, payer, recent_blockhash) = test.start().await;
        let new_account = Keypair::new();

        let memo_str = "a".repeat(128); // Maximum allowed length
        let ix_data = ExtendedSPLMemoInstruction::CreateCompressedMemo {
            memo: memo_str.to_string(),
        };

        let instruction = Instruction::new_with_borsh(program_id, &ix_data,vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(new_account.pubkey(), true),
            AccountMeta::new_readonly(solana_program::system_program::ID, false), // Add the system program
        ]);

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
            memo: memo_str,
        }
            .hash::<Poseidon>();
        assert_eq!(acct_data.data[..32], expected_hash.expect("BD"));
    }

    #[tokio::test]
    async fn test_create_compressed_memo_exceed_max_length() {
        let program_id = Pubkey::new_unique();

        let mut test = ProgramTest::new(
            "extanded_spl",
            program_id,
            processor!(process_instruction),
        );

        let (mut banks_client, payer, recent_blockhash) = test.start().await;
        let new_account = Keypair::new();

        let memo_str = "a".repeat(129); // Exceeds max length
        let ix_data = ExtendedSPLMemoInstruction::CreateCompressedMemo {
            memo: memo_str,
        };

        let instruction = Instruction::new_with_borsh(program_id, &ix_data,vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(new_account.pubkey(), true),
            AccountMeta::new_readonly(solana_program::system_program::ID, false), // Add the system program
        ]);

        let mut tx = Transaction::new_with_payer(&[instruction], Some(&payer.pubkey()));
        tx.sign(&[&payer, &new_account], recent_blockhash);

        let result = banks_client.process_transaction(tx).await;

        assert!(result.is_err());
        let err = result.unwrap_err().unwrap();

        assert_eq!(err, TransactionError::InstructionError(0, InstructionError::Custom(1)));
    }



    /// Runs a command to completion and returns stdout on success, or an error if it fails.
    fn run_command_and_get_stdout(cmd: &mut Command) -> Result<String, Box<dyn std::error::Error>> {
        println!("Running: {:?}", cmd);

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()) // Print stderr directly to our test console
            .spawn()?
            .wait_with_output()?;

        if !output.status.success() {
            return Err(format!(
                "Command {:?} failed with code {:?}",
                cmd, output.status.code()
            )
                .into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    pub struct TestValidator {
        ledger_dir: Arc<Mutex<String>>,
    }

    impl TestValidator {
        pub fn new() -> Self {
            let ledger_dir = Arc::new(Mutex::new(String::from("test-ledger")));
            let ledger_dir_clone = Arc::clone(&ledger_dir);

            // Setup Ctrl+C handler for cleanup
            ctrlc::set_handler(move || {
                println!("Interrupt signal received, cleaning up...");
                if let Ok(dir) = ledger_dir_clone.lock() {
                    println!("Removing ledger directory: {}", *dir);
                    if fs::metadata(&*dir).is_ok() {
                        if let Err(e) = fs::remove_dir_all(&*dir) {
                            eprintln!("Failed to remove ledger directory: {}", e);
                        }
                    }
                }
                process::exit(0);
            })
                .expect("Error setting Ctrl+C handler");

            Self { ledger_dir }
        }

        fn start_test_validator(&self) -> Result<(), Box<dyn std::error::Error + '_>> {
            // Create a unique directory for this test
            let unique_dir = format!("test-ledger-{}", rand::random::<u32>());
            {
                let mut dir = self.ledger_dir.lock()?;
                *dir = unique_dir.clone();
            }

            println!("Created test directory: {}", unique_dir);

            // Generate keypairs
            let keypairs = [
                ("validator-identity.json", "Validator Identity"),
                ("validator-vote-account.json", "Validator Vote Account"),
                ("validator-stake-account.json", "Validator Stake Account"),
                ("faucet-keypair.json", "Faucet Keypair"),
            ];

            for (file, description) in &keypairs {
                let path = format!("{}/{}", unique_dir, file);
                println!("Generating keypair: {}", path);
                let status = Command::new("solana-keygen")
                    .args(&["new", "--no-passphrase", "-so", &path])
                    .status()?;

                if !status.success() {
                    return Err(format!("Failed to generate {} keypair.", description).into());
                }
            }

            // Create the genesis ledger
            println!("Creating genesis ledger...");
            let status = Command::new("solana-genesis")
                .args(&[
                    "--hashes-per-tick", "sleep",
                    "--faucet-lamports", "500000000000000000",
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
                .status()?;

            if !status.success() {
                return Err("Failed to create genesis ledger.".into());
            }

            // Start the Solana Test Validator
            println!("Starting Solana Test Validator...");
            let status = Command::new("solana-test-validator")
                .args(&["--reset", "--ledger", &unique_dir])
                .status()?;

            if !status.success() {
                return Err("Failed to start Solana Test Validator.".into());
            }

            println!("Solana Test Validator is running with ledger: {}", unique_dir);
            Ok(())
        }

        pub fn spawn_validator_thread(self) -> Result<thread::JoinHandle<()>, Box<dyn std::error::Error>> {
            let arc_self = Arc::new(self);

            let handle = thread::spawn(move || {
                arc_self
                    .start_test_validator()
                    .expect("Failed to start validator");
            });

            thread::sleep(std::time::Duration::from_secs(5)); // todo add backoff

            Ok(handle)
        }
    }

    impl Drop for TestValidator { // todo add drop for handle
        fn drop(&mut self) {
            if let Ok(dir) = self.ledger_dir.lock() {
                println!("Cleaning up ledger directory: {}", *dir);
                if fs::metadata(&*dir).is_ok() {
                    if let Err(e) = fs::remove_dir_all(&*dir) {
                        eprintln!("Failed to remove ledger directory: {}", e);
                    }
                }
            }
        }
    }


    fn build_bpf_program(project_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Verify the project directory exists
        if !Path::new(project_dir).exists() {
            return Err(format!("Project directory '{}' does not exist", project_dir).into());
        }

        println!("Building the BPF program locally in {}...", project_dir);

        let status = Command::new("cargo")
            .args(&["build-bpf", "--manifest-path", &format!("{}/Cargo.toml", project_dir)])
            .status()?;

        if !status.success() {
            return Err("Failed to build BPF program locally".into());
        }

        println!("BPF build done at: {}", project_dir);
        Ok(())
    }


    /// 3) Deploy the resulting `.so` to the validator.
    ///    Assumes validator is listening on host port 8899.
    ///    Adjust your RPC URL if using Docker Desktop or Linux networking.
    fn deploy_program(project_dir: &str) -> Result<(), Box<dyn std::error::Error>> {

        // Verify the project directory exists
        if !Path::new(project_dir).exists() {
            return Err(format!("Project directory '{}' does not exist", project_dir).into());
        }

        println!("Ensuring a default keypair is created...");

        // Create a default keypair if it doesn't exist
        let keypair_path = "~/.config/solana/id.json";

        // Check if the keypair file exists
        if !Path::new(&keypair_path).exists() {
            println!("Creating default keypair...");
            let status = Command::new("solana-keygen")
                .args(&["new", "--no-passphrase", "-o", &keypair_path])
                .status()?;

            if !status.success() {
                return Err("Failed to create default Solana keypair.".into());
            }

            println!("Default keypair created successfully.");
        } else {
            println!("Default keypair already created.");
        }

        let output = Command::new("solana")
            .args(["address", "-k", keypair_path])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()?;

        if !output.status.success() {
            return Err("Failed to read keypair address with `solana address`".into());
        }

        let pubkey = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let status = Command::new("solana")
            .args([
                "airdrop",
                "10",
                &pubkey,
                "--url", "http://127.0.0.1:8899",
            ])
            .status()?;

        if !status.success() {
            return Err("Failed to airdrop SOL to the new wallet".into());
        }

        println!("Deploying the program using local Solana CLI tools...");

        // Define the RPC URL and the path to the program's `.so` file
        let rpc_url = "http://127.0.0.1:8899";
        let so_path = format!("{}/target/deploy/extanded_spl.so", project_dir);

        println!("Deploying program at: {}", so_path);

        // Check if the `.so` file exists
        if !Path::new(&so_path).exists() {
            return Err(format!("Program file '{}' does not exist. Build the program first.", so_path).into());
        }

        // Deploy the program using the Solana CLI
        let status = Command::new("solana")
            .args(&[
                "program",
                "deploy",
                "--keypair",
                &keypair_path,
                &so_path,
                "--url",
                rpc_url,
            ])
            .status()?;

        if !status.success() {
            return Err("Failed to deploy the program using Solana CLI.".into());
        }

        println!("Program deployed successfully.");
        Ok(())
    }

    /// 4) (Optional) Run a TypeScript test in a Node container.
    ///    Example: `npm install && npx ts-node tests/test_compressed_memo.ts`.
    fn run_typescript_test_locally(project_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(project_dir).exists() {
            return Err(format!("Project directory '{}' does not exist", project_dir).into());
        }

        println!("Running local TypeScript test in {}...", project_dir);

        {
            let status = Command::new("npm")
                .args(&["install"])
                .current_dir(project_dir)
                .status()?;

            if !status.success() {
                return Err("npm install failed".into());
            }
        }

        {
            let status = Command::new("npx")
                .args(&["ts-node", "tests/ts/test_compressed_memo.ts"])
                .current_dir(project_dir)
                .status()?;

            if !status.success() {
                return Err("ts-node test script failed".into());
            }
        }

        println!("TypeScript test completed successfully (locally).");
        Ok(())
    }

    /// 5) Stop & Remove the validator container.
    fn stop_and_remove_validator(name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let _ = Command::new("docker")
            .args(&["stop", name])
            .status();

        let _ = Command::new("docker")
            .args(&["rm", name])
            .status();

        Ok(())
    }

    /// Bring it all together in a single test.
    #[test]
    fn test_solana_program_fully_in_docker_via_commands() -> Result<(), Box<dyn std::error::Error>> {
        let validator_container_name = TestValidator::new();
        let handle = validator_container_name.spawn_validator_thread().expect("Failed to start validator");

        let project_dir = std::env::current_dir()?.to_string_lossy().to_string();
        build_bpf_program(&project_dir)?;

        deploy_program(&project_dir)?;
        println!("Program deployed.");

        run_typescript_test_locally(&project_dir)?;

        // 6) Cleanup
        // stop_and_remove_validator(&validator_container_name)?;
        println!("Validator stopped & removed.");


        Ok(())
    }


}
