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


    use std::{process::{Command, Stdio}, io::{self, Read}, thread};

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

    /// 1) Start Solana Test Validator in a container (detached),
    ///    mapping container port 8899 to host 8899, with `--reset`.
    ///    Forces `--platform linux/amd64` for Apple Silicon compatibility.
    // fn start_test_validator() -> Result<String, Box<dyn std::error::Error>> {
    //     let container_name = "solana-validator";
    //
    //     // Attempt to remove any existing container with the same name todo add check
    //     let _ = Command::new("docker")
    //         .args(&["rm", "-f", container_name])
    //         .status();
    //
    //     // Start container in detached mode
    //     run_command_and_get_stdout(
    //         Command::new("docker")
    //             .args(&[
    //                 "run",
    //                 "--platform", "linux/amd64",   // Force AMD64 emulation on Apple Silicon
    //                 "-d",                         // Detached mode
    //                 "--name", container_name,
    //                 "-p", "8899:8899",            // Map host 8899 to container 8899
    //                 "solanalabs/solana:v1.18.26",
    //                 "solana-test-validator",
    //                 "--reset",
    //             ])
    //     )?;
    //
    //     // Optionally sleep/wait a bit or parse logs
    //     std::thread::sleep(std::time::Duration::from_secs(5));
    //
    //     Ok(container_name.to_string())
    // }

    use std::fs;
    use std::sync::Arc;
    // fn start_test_validator() -> Result<(), Box<dyn std::error::Error>> {
    //     let ledger_dir = "test-ledger";
    //
    //     // Remove previous ledger directory if it exists
    //     if fs::metadata(ledger_dir).is_ok() {
    //         println!("Removing existing ledger directory: {}", ledger_dir);
    //         fs::remove_dir_all(ledger_dir)?;
    //     }
    //
    //     // Generate keypairs
    //     let keypairs = [
    //         ("validator-identity.json", "Validator Identity"),
    //         ("validator-vote-account.json", "Validator Vote Account"),
    //         ("validator-stake-account.json", "Validator Stake Account"),
    //         ("faucet-keypair.json", "Faucet Keypair"),
    //     ];
    //
    //     for (file, description) in &keypairs {
    //         println!("Generating {} keypair...", description);
    //         let status = Command::new("solana-keygen")
    //             .args(&["new", "--no-passphrase", "-so", file])
    //             .status()?;
    //
    //         if !status.success() {
    //             return Err(format!("Failed to generate {} keypair.", description).into());
    //         }
    //     }
    //
    //     // Create the genesis ledger
    //     println!("Creating genesis ledger...");
    //     let status = Command::new("solana-genesis")
    //         .args(&[
    //             "--hashes-per-tick", "sleep",
    //             "--faucet-lamports", "500000000000000000",
    //             "--bootstrap-validator", "validator-identity.json",
    //             "validator-vote-account.json",
    //             "validator-stake-account.json",
    //             "--faucet-pubkey", "faucet-keypair.json",
    //             "--ledger", ledger_dir,
    //             "--cluster-type", "development",
    //         ])
    //         .status()?;
    //
    //     if !status.success() {
    //         return Err("Failed to create genesis ledger.".into());
    //     }
    //
    //     // Start the Solana Test Validator
    //     println!("Starting Solana Test Validator...");
    //     let status = Command::new("solana-test-validator")
    //         .args(&["--reset"])
    //         .status()?;
    //
    //     if !status.success() {
    //         return Err("Failed to start Solana Test Validator.".into());
    //     }
    //
    //     println!("Solana Test Validator is running with ledger: {}", ledger_dir);
    //
    //     Ok(())
    // }


    use std::sync::Mutex;
    use std::process;
    use tokio::task::JoinHandle;

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




    /// 2) Build the BPF program using another ephemeral container.
    ///    Mount the local project directory so it can run `cargo build-bpf`.
    fn build_bpf_program(project_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        run_command_and_get_stdout(
            Command::new("docker")
                .args(&[
                    "run",
                    "--platform", "linux/amd64",  // Force AMD64 on Apple Silicon
                    "--rm",                       // Remove container when done
                    "-v", &format!("{}:/project:rw,z", project_dir),
                    "-w", "/project",
                    "solanalabs/solana:v1.18.26",
                    // "cargo", "build-bpf",
                    // "--manifest-path", "/project/Cargo.toml",
                    "solana-build",
                ])
        )?;

        Ok(())
    }

    /// 3) Deploy the resulting `.so` to the validator.
    ///    Assumes validator is listening on host port 8899.
    ///    Adjust your RPC URL if using Docker Desktop or Linux networking.
    fn deploy_program(project_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        // For Docker Desktop on macOS/Windows, "host.docker.internal" often works.
        // On Linux, you might need to use "127.0.0.1" or run with `--network host`.
        let rpc_url = "http://host.docker.internal:8899";

        // The `.so` is typically found at /project/target/deploy/extanded_spl.so
        let so_path = "/project/target/deploy/extanded_spl.so";

        run_command_and_get_stdout(
            Command::new("docker")
                .args(&[
                    "run",
                    "--platform", "linux/amd64",
                    "--rm",
                    "-v", &format!("{}:/project", project_dir),
                    "-w", "/project",
                    "solanalabs/solana:v1.18.26",
                    "solana",
                    "program",
                    "deploy",
                    so_path,
                    "--url",
                    rpc_url,
                ])
        )?;
        Ok(())
    }

    /// 4) (Optional) Run a TypeScript test in a Node container.
    ///    Example: `npm install && npx ts-node tests/test_compressed_memo.ts`.
    fn run_typescript_test(project_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        let rpc_url = "http://host.docker.internal:8899"; // Example
        let ts_script = "tests/test_compressed_memo.ts";

        run_command_and_get_stdout(
            Command::new("docker")
                .args(&[
                    "run",
                    "--platform", "linux/amd64",
                    "--rm",
                    "-v", &format!("{}:/project", project_dir),
                    "-w", "/project",
                    "node:18-alpine",
                    "sh",
                    "-c",
                    &format!("npm install && npx ts-node {}", ts_script),
                ])
        )?;
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
        // 1) Start the validator
        let validator_container_name = TestValidator::new();
        validator_container_name.spawn_validator_thread().expect("Failed to start validator");


        // 2) The local path to your Solana program project
        let project_dir = std::env::current_dir()?.to_string_lossy().to_string();

        // // 3) Build BPF program
        build_bpf_program(&project_dir)?;
        println!("BPF build done.");
        //
        // 4) Deploy program to validator
        // deploy_program(&project_dir)?;
        // println!("Program deployed.");

        // 5) (Optional) Run TypeScript test
        //    Uncomment to run TS logic
        // run_typescript_test(&project_dir)?;

        // 6) Cleanup
        // stop_and_remove_validator(&validator_container_name)?;
        println!("Validator stopped & removed.");

        Ok(())
    }


}
