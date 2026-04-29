use crucible_fuzzer::anchor_lang::system_program;
use crucible_fuzzer::*;
use escrow::*;
use solana_keypair::Keypair;
use solana_pubkey::Pubkey;
use solana_signer::Signer;
use std::rc::Rc;

const INITIAL_BALANCE: u64 = 10_000_000_000;
const UNLOCK_DELAY: u64 = 10;

#[derive(Clone)]
struct EscrowFixture {
    ctx: TestContext,
    program_id: Pubkey,
    depositor: Rc<Keypair>,
    beneficiary: Rc<Keypair>,
    vault_pda: Pubkey,
    unlock_slot: u64,
    successful_withdraw_slots: Vec<u64>,
}

#[fuzz_fixture]
impl EscrowFixture {
    pub fn setup() -> Self {
        let mut ctx = TestContext::new();
        let program_id = Pubkey::new_from_array(ID.to_bytes());
        ctx.add_program(&program_id, "../../target/deploy/escrow.so")
            .unwrap();

        let depositor = Rc::new(Keypair::new());
        let beneficiary = Rc::new(Keypair::new());
        for kp in [&depositor, &beneficiary] {
            ctx.create_account()
                .pubkey(kp.pubkey())
                .lamports(INITIAL_BALANCE)
                .owner(system_program::ID)
                .create()
                .unwrap();
        }

        let (vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", depositor.pubkey().as_ref()],
            &program_id,
        );
        let unlock_slot = ctx.slot() + UNLOCK_DELAY;

        ctx.program(program_id)
            .call(instruction::Initialize {
                beneficiary: beneficiary.pubkey(),
                unlock_slot,
            })
            .accounts(accounts::Initialize {
                vault: vault_pda,
                depositor: depositor.pubkey(),
                system_program: system_program::ID,
            })
            .signers(&[&*depositor])
            .send()
            .unwrap();

        Self {
            ctx,
            program_id,
            depositor,
            beneficiary,
            vault_pda,
            unlock_slot,
            successful_withdraw_slots: Vec::new(),
        }
    }

    pub fn action_deposit(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        self.ctx
            .program(self.program_id)
            .call(instruction::Deposit { amount })
            .accounts(accounts::Deposit {
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
                system_program: system_program::ID,
            })
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    pub fn action_withdraw(&mut self, #[range(1..1_000_000)] amount: u64) -> bool {
        let pre_slot = self.ctx.slot();
        let success = self
            .ctx
            .program(self.program_id)
            .call(instruction::Withdraw { amount })
            .accounts(accounts::Withdraw {
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
            })
            .signers(&[&*self.depositor])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false);
        if success {
            self.successful_withdraw_slots.push(pre_slot);
        }
        success
    }

    pub fn action_claim(&mut self) -> bool {
        self.ctx
            .program(self.program_id)
            .call(instruction::Claim {})
            .accounts(accounts::Claim {
                vault: self.vault_pda,
                depositor: self.depositor.pubkey(),
                beneficiary: self.beneficiary.pubkey(),
            })
            .signers(&[&*self.beneficiary])
            .send()
            .map(|o| o.is_success())
            .unwrap_or(false)
    }

    pub fn action_advance_slots(&mut self, #[range(0..15)] slots: u64) -> bool {
        let new_slot = self.ctx.slot() + slots;
        self.ctx.warp_to_slot(new_slot);
        true
    }
}

// Time-guard invariant: every successful withdraw must have happened strictly
// before unlock. The seeded bug uses `slot <= unlock_slot`, which lets a
// withdraw succeed at the exact unlock slot — the fuzzer should find that.
#[invariant_test]
fn invariant_escrow(fixture: &mut EscrowFixture) {
    for &s in &fixture.successful_withdraw_slots {
        fuzz_assert_lt!(
            s,
            fixture.unlock_slot,
            "withdraw at slot {} should have been rejected (unlock_slot = {})",
            s,
            fixture.unlock_slot
        );
    }
}
