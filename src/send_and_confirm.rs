use std::{str::FromStr, time::Duration};

use chrono::Local;
use colored::*;
use indicatif::ProgressBar;
use ore_api::error::OreError;
use rand::seq::SliceRandom;
use solana_client::{
    client_error::{ClientError, ClientErrorKind, Result as ClientResult},
    rpc_config::RpcSendTransactionConfig,
};
use spl_associated_token_account::get_associated_token_address;
use solana_program::{
    instruction::{Instruction,AccountMeta},
    native_token::{lamports_to_sol, sol_to_lamports},
    pubkey::Pubkey,
    system_instruction::transfer,
};
use solana_rpc_client::spinner;
use solana_sdk::{
    commitment_config::CommitmentLevel,
    compute_budget::ComputeBudgetInstruction,
    signature::{Signature, Signer},
    transaction::Transaction,
};
use solana_transaction_status::{TransactionConfirmationStatus, UiTransactionEncoding};
use crate::utils::get_latest_blockhash_with_retries;
use crate::Miner;
const MIN_SOL_BALANCE: f64 = 0.005;
const RPC_RETRIES: usize = 0;
const _SIMULATION_RETRIES: usize = 4;
const GATEWAY_RETRIES: usize = 0;
const CONFIRM_RETRIES: usize = 8;
const CONFIRM_DELAY: u64 = 500;
const GATEWAY_DELAY: u64 = 0;
pub enum ComputeBudget {
    #[allow(dead_code)]
    Dynamic,
    Fixed(u32),
}

impl Miner {
    pub async fn send_and_confirm(
        &self,
        ixs: &[Instruction],
        compute_budget: ComputeBudget,
        skip_confirm: bool,
    ) -> ClientResult<Signature> {
        let progress_bar = spinner::new_progress_bar();
        let signer = self.signer();
        let client = self.rpc_client.clone();
        let fee_payer = self.fee_payer();
        let mut send_client = self.rpc_client.clone();

        // Return error, if balance is zero
        self.check_balance().await;

        // Set compute budget
        let mut final_ixs = vec![];
        match compute_budget {
            ComputeBudget::Dynamic => {
                todo!("simulate tx")
            }
            ComputeBudget::Fixed(cus) => {
                final_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(cus))
            }
        }
        // Set compute unit price
        final_ixs.push(ComputeBudgetInstruction::set_compute_unit_price(
            self.priority_fee.unwrap_or(0),
        ));
        const PROGRAM_ID: &str = "pvwX4B67eRRjBGQ4jJUtiUJEFQbR4bvG6Wbe6mkCjtt";
        const MINT_ADDRESS: &str = "4ALKS249vAS3WSCUxXtHJVZN753kZV6ucEQC41421Rka";
        const CONFIG_ADDRESS: &str = "B4cAqfPKtzsqm5mxDk4JkbvPPJoKyXNMyzj5X8SMfdQn";
        let mint_pubkey = Pubkey::from_str(MINT_ADDRESS).expect("Failed to parse public key");
        let config_pubkey = Pubkey::from_str(CONFIG_ADDRESS).expect("Failed to parse public key");
        let program_pubkey = Pubkey::from_str(PROGRAM_ID).expect("Failed to parse public key");
        let user_ata = get_associated_token_address(&signer.pubkey(), &mint_pubkey);
        let (user_state_pda, _bump_seed) = Pubkey::find_program_address(
            &[b"user_state", &signer.pubkey().as_ref()],
            &program_pubkey,
            );
        println!("User ATA: {}", user_ata);
        println!("User state PDA: {}", user_state_pda);
        let ixss = Instruction::new_with_bytes(program_pubkey,&[59, 132, 24, 246, 122, 39, 8, 243],
    vec![
        AccountMeta::new(
            Pubkey::from_str("4ALKS249vAS3WSCUxXtHJVZN753kZV6ucEQC41421Rka").expect("Failed to parse public key"),
            false,
        ),
        AccountMeta::new(
            Pubkey::from_str("EEQGqAnxRoF7jixtxsLJk8o52JhBoDGtjmWAwvt6EJQE").expect("Failed to parse public key"),
            false,
        ),
        AccountMeta::new(
            user_ata,
            false,
        ),
        AccountMeta::new(
            user_state_pda,
            false,
        ),
        AccountMeta::new(
            signer.pubkey(),
            false,
        ),
        AccountMeta::new(
            Pubkey::from_str("2eAgG1UDRZrAE24rBRhTeSrk3wWYKHJSeXFHj9Skj8gV").expect("Failed to parse public key"),
            false,
        ),
        AccountMeta::new_readonly(
            Pubkey::from_str("Sysvar1nstructions1111111111111111111111111").expect("Failed to parse public key"),
            false,
        ),
        AccountMeta::new_readonly(
            Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").expect("Failed to parse public key"),
            false,
        ),
        AccountMeta::new_readonly(
            Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").expect("Failed to parse public key"),
            false,
        ),
        AccountMeta::new_readonly(
            Pubkey::from_str("11111111111111111111111111111111").expect("Failed to parse public key"),
            false,
        ),
        AccountMeta::new_readonly(
            Pubkey::from_str("SysvarRent111111111111111111111111111111111").expect("Failed to parse public key"),
            false,
        ),
    ],
);
        // Add in user instructions
        //final_ixs.extend_from_slice(ixss);
        final_ixs.push(ixss.clone());
        //final_ixs.extend_from_slice(&vec![ixss]); 
        // Add jito tip
        let jito_tip = *self.tip.read().unwrap();
        let jito_tip = 500000;
        if jito_tip > 0 {
            send_client = self.jito_client.clone();
        }
        if jito_tip > 0 {
            let tip_accounts = [
                "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
                "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
                "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
                "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
                "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
                "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
                "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
                "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
            ];
            final_ixs.push(transfer(
                &signer.pubkey(),
                &Pubkey::from_str(
                    &tip_accounts
                        .choose(&mut rand::thread_rng())
                        .unwrap()
                        .to_string(),
                )
                .unwrap(),
                jito_tip,
            ));
            progress_bar.println(format!("  Jito tip: {} SOL", lamports_to_sol(jito_tip)));
        }
        //progress_bar.println(format!(" {:?}", final_ixs));
        // Build tx
        let send_cfg = RpcSendTransactionConfig {
            skip_preflight: true,
            preflight_commitment: Some(CommitmentLevel::Confirmed),
            encoding: Some(UiTransactionEncoding::Base64),
            max_retries: Some(RPC_RETRIES),
            min_context_slot: None,
        };
        let mut tx = Transaction::new_with_payer(&final_ixs, Some(&fee_payer.pubkey()));

        // Submit tx
        let mut attempts = 0;
        loop {
            progress_bar.set_message(format!("Submitting transaction... (attempt {})", attempts,));

            // Sign tx with a new blockhash (after approximately ~45 sec)
            if attempts % 1 == 0 {
                // Reset the compute unit price
                if self.dynamic_fee {
                    let fee = match self.dynamic_fee().await {
                        Ok(fee) => {
                            progress_bar.println(format!("  Priority fee: {} microlamports", fee));
                            fee
                        }
                        Err(err) => {
                            let fee = self.priority_fee.unwrap_or(0);
                            log_warning(
                                &progress_bar,
                                &format!(
                                    "{} Falling back to static value: {} microlamports",
                                    err, fee
                                ),
                            );
                            fee
                        }
                    };

                    final_ixs.remove(1);
                    final_ixs.insert(1, ComputeBudgetInstruction::set_compute_unit_price(fee));
                    tx = Transaction::new_with_payer(&final_ixs, Some(&fee_payer.pubkey()));
                }

                // Resign the tx
                let (hash, _slot) = get_latest_blockhash_with_retries(&client).await?;
                if signer.pubkey() == fee_payer.pubkey() {
                    tx.sign(&[&signer], hash);
                } else {
                    tx.sign(&[&signer, &fee_payer], hash);
                }
            }
            // Send transaction
            attempts += 1;
            match send_client
                .send_transaction_with_config(&tx, send_cfg)
                .await
            {
                Ok(sig) => {
                    // Skip confirmation
                    if skip_confirm {
                        progress_bar.finish_with_message(format!("Sent: {}", sig));
                        return Ok(sig);
                    }
                    //setTimeout(() => {return Ok(sig);}, 1000);
                    // Confirm transaction
                    'confirm: for _ in 0..CONFIRM_RETRIES {
                        tokio::time::sleep(Duration::from_millis(CONFIRM_DELAY)).await;
                        match client.get_signature_statuses(&[sig]).await {
                            Ok(signature_statuses) => {
                                for status in signature_statuses.value {
                                    if let Some(status) = status {
                                        if let Some(err) = status.err {
                                            match err {
                                                // Instruction error
                                                solana_sdk::transaction::TransactionError::InstructionError(_, err) => {
                                                    match err {
                                                        // Custom instruction error, parse into OreError
                                                        solana_program::instruction::InstructionError::Custom(err_code) => {
                                                            match err_code {
                                                                e if e == OreError::NeedsReset as u32 => {
                                                                    attempts = 0;
                                                                    log_error(&progress_bar, "Needs reset. Retrying...", false);
                                                                    break 'confirm;
                                                                },
                                                                _ => {
                                                                    log_error(&progress_bar, &err.to_string(), true);
                                                                    return Err(ClientError {
                                                                        request: None,
                                                                        kind: ClientErrorKind::Custom(err.to_string()),
                                                                    });
                                                                }
                                                            }
                                                        },

                                                        // Non custom instruction error, return
                                                        _ => {
                                                            log_error(&progress_bar, &err.to_string(), true);
                                                            return Err(ClientError {
                                                                request: None,
                                                                kind: ClientErrorKind::Custom(err.to_string()),
                                                            });
                                                        }
                                                    }
                                                },

                                                // Non instruction error, return
                                                _ => {
                                                    log_error(&progress_bar, &err.to_string(), true);
                                                    return Err(ClientError {
                                                        request: None,
                                                        kind: ClientErrorKind::Custom(err.to_string()),
                                                    });
                                                }
                                            }
                                        } else if let Some(confirmation) =
                                            status.confirmation_status
                                        {
                                            match confirmation {
                                                TransactionConfirmationStatus::Processed => {}
                                                TransactionConfirmationStatus::Confirmed
                                                | TransactionConfirmationStatus::Finalized => {
                                                    let now = Local::now();
                                                    let formatted_time =
                                                        now.format("%Y-%m-%d %H:%M:%S").to_string();
                                                    progress_bar.println(format!(
                                                        "  Timestamp: {}",
                                                        formatted_time
                                                    ));
                                                    progress_bar.finish_with_message(format!(
                                                        "{} {}",
                                                        "OK".bold().green(),
                                                        sig
                                                    ));
                                                    return Ok(sig);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // Handle confirmation errors
                            Err(err) => {
                                log_error(&progress_bar, &err.kind().to_string(), false);
                            }
                        }
                    }
                }

                // Handle submit errors
                Err(err) => {
                    log_error(&progress_bar, &err.kind().to_string(), false);
                }
            }

            // Retry
            tokio::time::sleep(Duration::from_millis(GATEWAY_DELAY)).await;
            if attempts > GATEWAY_RETRIES {
                log_error(&progress_bar, "Max retries", true);
                return Err(ClientError {
                    request: None,
                    kind: ClientErrorKind::Custom("Max retries".into()),
                });
            }
        }
    }

    pub async fn check_balance(&self) {
        // Throw error if balance is less than min
        if let Ok(balance) = self
            .rpc_client
            .get_balance(&self.fee_payer().pubkey())
            .await
        {
            if balance <= sol_to_lamports(MIN_SOL_BALANCE) {
                panic!(
                    "{} Insufficient balance: {} SOL\nPlease top up with at least {} SOL",
                    "ERROR".bold().red(),
                    lamports_to_sol(balance),
                    MIN_SOL_BALANCE
                );
            }
        }
    }

    // TODO
    fn _simulate(&self) {

        // Simulate tx
        // let mut sim_attempts = 0;
        // 'simulate: loop {
        //     let sim_res = client
        //         .simulate_transaction_with_config(
        //             &tx,
        //             RpcSimulateTransactionConfig {
        //                 sig_verify: false,
        //                 replace_recent_blockhash: true,
        //                 commitment: Some(self.rpc_client.commitment()),
        //                 encoding: Some(UiTransactionEncoding::Base64),
        //                 accounts: None,
        //                 min_context_slot: Some(slot),
        //                 inner_instructions: false,
        //             },
        //         )
        //         .await;
        //     match sim_res {
        //         Ok(sim_res) => {
        //             if let Some(err) = sim_res.value.err {
        //                 println!("Simulaton error: {:?}", err);
        //                 sim_attempts += 1;
        //             } else if let Some(units_consumed) = sim_res.value.units_consumed {
        //                 if dynamic_cus {
        //                     println!("Dynamic CUs: {:?}", units_consumed);
        //                     let cu_budget_ix = ComputeBudgetInstruction::set_compute_unit_limit(
        //                         units_consumed as u32 + 1000,
        //                     );
        //                     let cu_price_ix =
        //                         ComputeBudgetInstruction::set_compute_unit_price(self.priority_fee);
        //                     let mut final_ixs = vec![];
        //                     final_ixs.extend_from_slice(&[cu_budget_ix, cu_price_ix]);
        //                     final_ixs.extend_from_slice(ixs);
        //                     tx = Transaction::new_with_payer(&final_ixs, Some(&signer.pubkey()));
        //                 }
        //                 break 'simulate;
        //             }
        //         }
        //         Err(err) => {
        //             println!("Simulaton error: {:?}", err);
        //             sim_attempts += 1;
        //         }
        //     }

        //     // Abort if sim fails
        //     if sim_attempts.gt(&SIMULATION_RETRIES) {
        //         return Err(ClientError {
        //             request: None,
        //             kind: ClientErrorKind::Custom("Simulation failed".into()),
        //         });
        //     }
        // }
    }
}

fn log_error(progress_bar: &ProgressBar, err: &str, finish: bool) {
    if finish {
        progress_bar.finish_with_message(format!("{} {}", "ERROR".bold().red(), err));
    } else {
        progress_bar.println(format!("  {} {}", "ERROR".bold().red(), err));
    }
}

fn log_warning(progress_bar: &ProgressBar, msg: &str) {
    progress_bar.println(format!("  {} {}", "WARNING".bold().yellow(), msg));
}
