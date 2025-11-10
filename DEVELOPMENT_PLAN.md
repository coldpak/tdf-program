# State, Account, Instruction 개발 계획

## 📋 개발 단계별 계획

### Phase 1: 요구사항 정의 및 설계 (Design Phase)

#### 1.1 비즈니스 로직 정의
- [ ] 어떤 데이터를 저장할지 결정 (State 정의)
- [ ] 어떤 기능이 필요한지 나열 (Instruction 목록)
- [ ] State 간 관계 파악 (1:1, 1:N, N:M 등)
- [ ] Ephemeral Rollup에서 실행할 로직 vs Base Layer에서 실행할 로직 구분

#### 1.2 State 구조 설계
```rust
// 예시: 어떤 데이터 구조가 필요한지 설계
#[account]
pub struct YourState {
    // 필요한 필드들 정의
    pub field1: u64,
    pub field2: Pubkey,
    // ...
}
```

**체크리스트:**
- [ ] State에 필요한 모든 필드 식별
- [ ] 각 필드의 타입 결정 (u64, Pubkey, Vec, 등)
- [ ] Space 계산 (8 bytes discriminator + 필드 크기)
- [ ] PDA seeds 결정 (필요한 경우)

---

### Phase 2: State 정의 (State Definition)

#### 2.1 State 구조체 작성
```rust
#[account]
pub struct YourState {
    pub field1: u64,           // 8 bytes
    pub field2: Pubkey,        // 32 bytes
    pub field3: Vec<Item>,     // 4 bytes (length) + items
}

// 추가 State가 필요한 경우
#[account]
pub struct AnotherState {
    // ...
}
```

#### 2.2 Constants 정의
```rust
// PDA seeds 정의
pub const YOUR_STATE_SEED: &[u8] = b"your-state-seed";
pub const ANOTHER_SEED: &[u8] = b"another-seed";
```

**체크리스트:**
- [ ] 모든 State 구조체 정의
- [ ] `#[account]` 어노테이션 확인
- [ ] Space 계산 검증
- [ ] Seeds 정의

---

## 🔄 Ephemeral Rollup 사이클: Delegation → Commit → Undelegation

### 핵심 개념

Ephemeral Rollup (ER)은 Solana의 **Base Layer**와 별도로 운영되는 **Layer 2** 환경입니다. 
프로그램이 ER에서 실행되려면 다음 사이클을 이해하고 구현해야 합니다:

```
1. Delegate (위임) → Account를 ER에 위임
2. Operations (작업) → ER에서 State 조작
3. Commit (커밋) → ER의 변경사항을 Base Layer로 동기화
4. Undelegate (위임 해제) → ER에서 Base Layer로 완전히 복귀
```

---

### Phase 0: Ephemeral Rollup 매크로 및 함수 이해

#### 0.1 `#[ephemeral]` 매크로

**위치**: 프로그램 모듈 레벨에 적용
**역할**: 프로그램이 Ephemeral Rollup과 호환되도록 설정

```rust
#[ephemeral]
#[program]
pub mod your_program {
    // 모든 instructions는 ER에서도 실행 가능
}
```

**주요 특징**:
- 프로그램이 ER에서 실행 가능하도록 설정
- Base Layer와 ER 양쪽에서 동작
- 특별한 Account 추가 없음 (프로그램 레벨 설정)

---

#### 0.2 `#[delegate]` 매크로 분석

**위치**: Account 구조체에 적용
**역할**: Account를 ER에 위임하기 위한 Account 구조체 자동 생성

**매크로가 자동으로 추가하는 것들**:

1. **`del` 어트리뷰트가 있는 필드에 대해**:
   - `buffer_{field_name}`: Buffer account (PDA)
   - `delegation_record_{field_name}`: Delegation record account (PDA)
   - `delegation_metadata_{field_name}`: Delegation metadata account (PDA)

2. **필수 필드들** (없으면 자동 추가):
   - `owner_program`: AccountInfo (프로그램 ID로 자동 설정)
   - `delegation_program`: AccountInfo (ephemeral-rollups-sdk ID로 자동 설정)
   - `system_program`: Program<System>

3. **자동 생성 메서드**:
   - `delegate_{field_name}(payer, seeds, config)` 메서드

**예시**:
```rust
#[delegate]
#[derive(Accounts)]
pub struct DelegateYourState<'info> {
    pub payer: Signer<'info>,
    #[account(mut, del)]  // ← 이 어트리뷰트가 핵심!
    /// CHECK: the correct pda
    pub pda: AccountInfo<'info>,
    // 아래는 자동으로 추가됨:
    // - buffer_pda: AccountInfo<'info>
    // - delegation_record_pda: AccountInfo<'info>
    // - delegation_metadata_pda: AccountInfo<'info>
    // - owner_program: AccountInfo<'info>
    // - delegation_program: AccountInfo<'info>
    // - system_program: Program<'info, System>
}

// 사용법:
pub fn delegate(ctx: Context<DelegateYourState>) -> Result<()> {
    // 자동 생성된 메서드 사용
    ctx.accounts.delegate_pda(
        &ctx.accounts.payer,
        &[YOUR_SEED],  // PDA seeds
        DelegateConfig {
            commit_frequency_ms: 30_000,  // 자동 commit 주기 (밀리초)
            validator: Some(pubkey!("...")),  // 선택적: 특정 validator 지정
        },
    )?;
    Ok(())
}
```

**DelegateConfig 옵션**:
- `commit_frequency_ms`: ER에서 Base Layer로 자동 commit 주기 (밀리초)
- `validator`: 선택적, 특정 validator 지정 (None이면 가장 가까운 validator 선택)

**체크리스트**:
- [ ] `#[delegate]` 어노테이션 추가
- [ ] 위임할 PDA 필드에 `del` 어트리뷰트 추가
- [ ] `DelegateConfig` 설정 (commit_frequency_ms, validator)
- [ ] PDA seeds 정확히 전달

---

#### 0.3 `#[commit]` 매크로 분석

**위치**: Account 구조체에 적용
**역할**: ER에서 Base Layer로 commit하기 위한 Account 구조체 자동 생성

**매크로가 자동으로 추가하는 것들**:

1. **필수 필드들** (없으면 자동 추가):
   - `magic_program`: Program<MagicProgram> (Magic Program)
   - `magic_context`: AccountInfo (Magic Context PDA, 자동으로 고정 주소)

**예시 - Commit without Action**:
```rust
#[commit]
#[derive(Accounts)]
pub struct CommitYourState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [YOUR_SEED], bump)]
    pub your_state: Account<'info, YourState>,
    // 아래는 자동으로 추가됨:
    // - magic_program: Program<'info, MagicProgram>
    // - magic_context: AccountInfo<'info>
}

// 사용법 1: 단순 commit (commit_accounts 사용)
pub fn commit(ctx: Context<CommitYourState>) -> Result<()> {
    use ephemeral_rollups_sdk::ephem::commit_accounts;
    
    commit_accounts(
        &ctx.accounts.payer,
        vec![&ctx.accounts.your_state.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;
    Ok(())
}

// 사용법 2: Commit + Undelegate (commit_and_undelegate_accounts 사용)
pub fn commit_and_undelegate(ctx: Context<CommitYourState>) -> Result<()> {
    use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;
    
    commit_and_undelegate_accounts(
        &ctx.accounts.payer,
        vec![&ctx.accounts.your_state.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;
    Ok(())
}
```

**예시 - Commit with Action**:
```rust
#[commit]
#[derive(Accounts)]
pub struct CommitWithAction<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(mut, seeds = [STATE_SEED], bump)]
    pub state_account: Account<'info, YourState>,
    
    /// CHECK: Target account for handler
    #[account(seeds = [TARGET_SEED], bump)]
    pub target_account: UncheckedAccount<'info>,
    
    /// CHECK: Your program ID
    pub program_id: AccountInfo<'info>,
    // 아래는 자동으로 추가됨:
    // - magic_program: Program<'info, MagicProgram>
    // - magic_context: AccountInfo<'info>
}

// 사용법: MagicInstructionBuilder 사용
pub fn commit_with_action(ctx: Context<CommitWithAction>) -> Result<()> {
    // ... ActionArgs, CallHandler 생성 ...
    
    let magic_builder = MagicInstructionBuilder {
        payer: ctx.accounts.payer.to_account_info(),
        magic_context: ctx.accounts.magic_context.to_account_info(),
        magic_program: ctx.accounts.magic_program.to_account_info(),
        magic_action: MagicAction::Commit(CommitType::WithHandler {
            commited_accounts: vec![ctx.accounts.state_account.to_account_info()],
            call_handlers: vec![call_handler],
        }),
    };
    
    magic_builder.build_and_invoke()?;
    Ok(())
}
```

**주요 함수들**:

1. **`commit_accounts`**: 단순 commit만 수행
   ```rust
   commit_accounts(
       payer,
       account_infos,  // commit할 account들
       magic_context,
       magic_program,
   )
   ```

2. **`commit_and_undelegate_accounts`**: Commit + Undelegate 동시 수행
   ```rust
   commit_and_undelegate_accounts(
       payer,
       account_infos,  // commit할 account들
       magic_context,
       magic_program,
   )
   ```

3. **`MagicInstructionBuilder`**: Commit + Action (복잡한 로직)
   - `CommitType::Standalone`: 단순 commit
   - `CommitType::WithHandler`: Commit + Handler 실행
   - `CommitAndUndelegate`: Commit + Undelegate + Handler 실행

**체크리스트**:
- [ ] `#[commit]` 어노테이션 추가
- [ ] Commit할 account들 식별
- [ ] 단순 commit vs commit + action vs commit + undelegate 결정
- [ ] Handler가 필요한 경우 ActionArgs, CallHandler 준비

---

#### 0.4 Delegate, Commit, Undelegate 사이클 플로우

**전체 플로우**:

```
┌─────────────────────────────────────────────────────────────┐
│                    BASE LAYER (Solana)                       │
│                                                               │
│  [Initialize] → Account 생성                                 │
│       ↓                                                       │
│  [Delegate] → Account를 ER에 위임                            │
│       ↓                                                       │
│  ────────────────────────────────────────────────────────────│
│                                                               │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │         EPHEMERAL ROLLUP (ER)                          │ │
│  │                                                          │ │
│  │  [Operations] → State 조작 (빠르고 저렴)               │ │
│  │       ↓                                                  │ │
│  │  [Commit] → 변경사항을 Base Layer로 동기화             │ │
│  │       ↓                                                  │ │
│  │  [Undelegate] → ER에서 제거, Base Layer로 복귀         │ │
│  └─────────────────────────────────────────────────────────┘ │
│                                                               │
│  [Operations] → Base Layer에서도 동작 가능                   │
└─────────────────────────────────────────────────────────────┘
```

**상세 단계별 가이드**:

#### Step 1: Initialize (Base Layer)
```rust
// Base Layer에서 State 초기화
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    ctx.accounts.your_state.field = 0;
    Ok(())
}
```

#### Step 2: Delegate (Base Layer → ER)
```rust
// Account를 ER에 위임
#[delegate]
#[derive(Accounts)]
pub struct DelegateYourState<'info> {
    pub payer: Signer<'info>,
    #[account(mut, del)]
    pub pda: AccountInfo<'info>,
}

pub fn delegate(ctx: Context<DelegateYourState>) -> Result<()> {
    ctx.accounts.delegate_pda(
        &ctx.accounts.payer,
        &[YOUR_SEED],
        DelegateConfig {
            commit_frequency_ms: 30_000,  // 30초마다 자동 commit
            validator: None,  // 가장 가까운 validator 자동 선택
        },
    )?;
    Ok(())
}
```

**Delegate 이후**:
- Account가 ER로 이동
- 이후 모든 operations는 ER에서 실행 (빠르고 저렴)
- `commit_frequency_ms` 주기로 자동 commit 가능

#### Step 3: Operations (ER 내)
```rust
// ER에서 State 조작 (Base Layer와 동일한 코드)
pub fn update(ctx: Context<Update>, value: u64) -> Result<()> {
    ctx.accounts.your_state.field = value;
    Ok(())
}
```

**ER에서의 장점**:
- 빠른 처리 속도
- 낮은 거래 수수료
- 높은 처리량

#### Step 4: Commit (ER → Base Layer)
```rust
// ER의 변경사항을 Base Layer로 동기화
#[commit]
#[derive(Accounts)]
pub struct CommitYourState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [YOUR_SEED], bump)]
    pub your_state: Account<'info, YourState>,
}

// 옵션 1: 단순 commit
pub fn commit(ctx: Context<CommitYourState>) -> Result<()> {
    commit_accounts(
        &ctx.accounts.payer,
        vec![&ctx.accounts.your_state.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;
    Ok(())
}

// 옵션 2: Commit + Undelegate
pub fn commit_and_undelegate(ctx: Context<CommitYourState>) -> Result<()> {
    commit_and_undelegate_accounts(
        &ctx.accounts.payer,
        vec![&ctx.accounts.your_state.to_account_info()],
        &ctx.accounts.magic_context,
        &ctx.accounts.magic_program,
    )?;
    Ok(())
}

// 옵션 3: Commit + Action (Handler 실행)
pub fn commit_with_action(ctx: Context<CommitWithAction>) -> Result<()> {
    // ... MagicInstructionBuilder 사용 ...
}
```

**Commit 옵션 비교**:

| 옵션 | 함수 | Delegate 상태 | Handler 실행 |
|------|------|---------------|--------------|
| 단순 Commit | `commit_accounts` | 유지 | ❌ |
| Commit + Undelegate | `commit_and_undelegate_accounts` | 해제 | ❌ |
| Commit + Action | `MagicInstructionBuilder` | 유지 | ✅ |
| Commit + Undelegate + Action | `MagicInstructionBuilder` | 해제 | ✅ |

#### Step 5: Undelegate (ER → Base Layer)
```rust
// ER에서 Base Layer로 완전히 복귀
// commit_and_undelegate_accounts 사용하거나
// MagicInstructionBuilder에서 CommitAndUndelegate 사용
```

**Undelegate 이후**:
- Account가 Base Layer로 완전히 복귀
- 이후 모든 operations는 Base Layer에서 실행

---

#### 0.5 매크로별 Account 자동 추가 요약

| 매크로 | 자동 추가 Account | 필수 필드 |
|--------|------------------|-----------|
| `#[ephemeral]` | 없음 (프로그램 레벨) | 없음 |
| `#[delegate]` | `buffer_{field}`, `delegation_record_{field}`, `delegation_metadata_{field}` | `owner_program`, `delegation_program`, `system_program` |
| `#[commit]` | 없음 | `magic_program`, `magic_context` |

**주의사항**:
- `#[delegate]`의 `del` 어트리뷰트가 있는 필드에 대해서만 추가 Account 생성
- `#[commit]`는 항상 `magic_program`과 `magic_context` 추가
- 매크로가 자동으로 추가하는 Account들은 명시적으로 선언 불필요

---

#### 0.6 사이클별 주의사항

**Delegate 시**:
- ✅ `del` 어트리뷰트 정확히 지정
- ✅ PDA seeds 정확히 전달
- ✅ `DelegateConfig` 설정 (commit_frequency_ms, validator)
- ⚠️ Delegate 후 즉시 ER에서 작업 가능하지만 네트워크 지연 고려

**Commit 시**:
- ✅ Commit할 account들 정확히 지정
- ✅ 단순 commit vs commit + action vs commit + undelegate 선택
- ✅ Handler 사용 시 Account 순서, Writable 설정 확인
- ⚠️ Commit은 비용이 발생할 수 있음

**Undelegate 시**:
- ✅ Commit 후 Undelegate 또는 동시 수행
- ✅ Undelegate 후 Base Layer에서 작업 가능
- ⚠️ Undelegate는 영구적이므로 신중하게 결정

---

### Phase 3: Instruction 정의 (Instruction Logic)

#### 3.1 기본 Instruction 작성 순서
1. **Initialize Instruction** (초기화)
2. **CRUD Instructions** (Create, Read, Update, Delete)
3. **Business Logic Instructions** (도메인 특화 로직)

#### 3.2 Instruction 패턴

**패턴 1: 기본 Instruction (Ephemeral Rollup 내에서 실행)**
```rust
#[ephemeral]
#[program]
pub mod your_program {
    pub fn your_instruction(ctx: Context<YourAccounts>, params: YourParams) -> Result<()> {
        // 로직 구현
        Ok(())
    }
}
```

**패턴 2: Initialize Instruction**
```rust
pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
    let state = &mut ctx.accounts.your_state;
    state.field1 = 0;
    state.field2 = ctx.accounts.user.key();
    Ok(())
}
```

**패턴 3: Update Instruction**
```rust
pub fn update(ctx: Context<Update>, new_value: u64) -> Result<()> {
    let state = &mut ctx.accounts.your_state;
    state.field1 = new_value;
    Ok(())
}
```

**패턴 4: Magic Action (Commit + Base Layer Handler)**
```rust
pub fn commit_with_action(ctx: Context<CommitWithAction>) -> Result<()> {
    // 1. Instruction 데이터 준비
    let instruction_data = anchor_lang::InstructionData::data(
        &crate::instruction::YourHandler {}
    );

    // 2. ActionArgs 생성
    let action_args = ActionArgs {
        escrow_index: 0,
        data: instruction_data,
    };
    
    // 3. Account 메타데이터 준비
    let accounts = vec![
        ShortAccountMeta {
            pubkey: ctx.accounts.target_account.key(),
            is_writable: true,
        },
        // 필요한 계정들 추가
    ];

    // 4. CallHandler 생성
    let call_handler = CallHandler {
        args: action_args,
        compute_units: 200_000,
        escrow_authority: ctx.accounts.payer.to_account_info(),
        destination_program: crate::ID,
        accounts,
    };

    // 5. MagicInstructionBuilder로 실행
    let magic_builder = MagicInstructionBuilder {
        payer: ctx.accounts.payer.to_account_info(),
        magic_context: ctx.accounts.magic_context.to_account_info(),
        magic_program: ctx.accounts.magic_program.to_account_info(),
        magic_action: MagicAction::Commit(CommitType::WithHandler {
            commited_accounts: vec![ctx.accounts.state_account.to_account_info()],
            call_handlers: vec![call_handler],
        }),
    };
    
    magic_builder.build_and_invoke()?;
    Ok(())
}
```

**체크리스트:**
- [ ] Initialize instruction
- [ ] 기본 CRUD instructions
- [ ] 비즈니스 로직 instructions
- [ ] Magic Action이 필요한 경우 handler instruction 정의

---

### Phase 4: Account Validation 정의 (Account Constraints)

#### 4.1 Account 구조체 패턴

**패턴 1: 기본 Initialize**
```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init_if_needed, payer = user, space = 8 + 8, seeds = [YOUR_SEED], bump)]
    pub your_state: Account<'info, YourState>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}
```

**패턴 2: 일반 Update (Ephemeral Rollup 내)**
```rust
#[derive(Accounts)]
pub struct Update<'info> {
    #[account(mut, seeds = [YOUR_SEED], bump)]
    pub your_state: Account<'info, YourState>,
}
```

**패턴 3: Magic Action Handler (Base Layer에서 실행)**
```rust
#[derive(Accounts)]
pub struct YourHandler<'info> {
    #[account(mut, seeds = [TARGET_SEED], bump)]
    pub target_state: Account<'info, TargetState>,
    /// CHECK: Committed account from ER
    pub committed_account: UncheckedAccount<'info>,
    /// CHECK: Escrow account
    pub escrow: UncheckedAccount<'info>,
    /// CHECK: Escrow authority
    pub escrow_auth: UncheckedAccount<'info>,
}
```

**패턴 4: Delegate (Ephemeral Rollup 위임)**
```rust
#[delegate]
#[derive(Accounts)]
pub struct DelegateYourState<'info> {
    pub payer: Signer<'info>,
    #[account(mut, del)]
    /// CHECK: the correct pda
    pub pda: AccountInfo<'info>,
}
```

**패턴 5: Commit (Commit without Action)**
```rust
#[commit]
#[derive(Accounts)]
pub struct CommitYourState<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [YOUR_SEED], bump)]
    pub your_state: Account<'info, YourState>,
}
```

**패턴 6: Commit with Action**
```rust
#[commit]
#[derive(Accounts)]
pub struct CommitWithAction<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(mut, seeds = [STATE_SEED], bump)]
    pub state_account: Account<'info, YourState>,

    /// CHECK: Target account for handler - not mut here, writable set in handler
    #[account(seeds = [TARGET_SEED], bump)]
    pub target_account: UncheckedAccount<'info>,

    /// CHECK: Your program ID
    pub program_id: AccountInfo<'info>,
}
```

**체크리스트:**
- [ ] 각 Instruction에 대한 Account 구조체 정의
- [ ] 적절한 constraint 사용 (`init_if_needed`, `mut`, `seeds`, `bump` 등)
- [ ] Magic Action handler의 Account 구조체 정의
- [ ] Delegate/Commit 어노테이션 필요한 경우 추가

---

### Phase 5: Handler Instruction 정의 (Base Layer Handler)

#### 5.1 Handler Instruction 작성
```rust
// Base Layer에서 실행될 Handler
pub fn your_handler(ctx: Context<YourHandler>) -> Result<()> {
    // ER에서 commit된 데이터를 읽어서 처리
    let committed_info = &ctx.accounts.committed_account.to_account_info();
    let mut data: &[u8] = &committed_info.try_borrow_data()?;
    let committed_state = YourState::try_deserialize(&mut data)?;
    
    // Base Layer의 state 업데이트
    let target_state = &mut ctx.accounts.target_state;
    target_state.field = committed_state.field;
    
    Ok(())
}
```

**체크리스트:**
- [ ] Handler instruction 정의
- [ ] Committed account에서 데이터 읽기 로직
- [ ] Base Layer state 업데이트 로직

---

### Phase 6: 테스트 및 검증

#### 6.1 Unit Testing
- [ ] 각 Instruction의 로직 테스트
- [ ] Account validation 테스트
- [ ] Edge case 처리 확인

#### 6.2 Integration Testing
- [ ] Ephemeral Rollup에서 실행 테스트
- [ ] Magic Action 동작 테스트
- [ ] Commit/Delegate/Undelegate 플로우 테스트

---

## 🎯 핵심 체크리스트

### State 설계
- [ ] State 구조체 정의
- [ ] Space 계산 정확히
- [ ] PDA seeds 정의

### Instruction 구현
- [ ] Initialize instruction
- [ ] CRUD instructions
- [ ] Magic Action instructions (필요한 경우)
- [ ] Handler instructions (Base Layer)

### Account Validation
- [ ] 각 Instruction의 Account 구조체
- [ ] 적절한 constraints 적용
- [ ] Magic Action handler의 Account 구조

### ER 사이클 구현
- [ ] `#[ephemeral]` 매크로 프로그램 레벨에 추가
- [ ] `#[delegate]` Account 구조체 정의 및 구현
- [ ] `#[commit]` Account 구조체 정의 및 구현
- [ ] Delegate → Operations → Commit → Undelegate 플로우 구현

### 테스트
- [ ] 로컬 테스트
- [ ] Ephemeral Rollup 테스트
- [ ] Delegate 테스트
- [ ] ER에서 Operations 테스트
- [ ] Commit 테스트
- [ ] Undelegate 테스트
- [ ] Magic Action 동작 확인
- [ ] 전체 ER 사이클 통합 테스트

---

## 📝 개발 순서 권장사항

### 기본 개발 순서

1. **Phase 0: ER 매크로 및 사이클 이해** → ER의 delegation, commit, undelegation 사이클 완전히 이해
2. **Phase 1: 요구사항 정의 및 설계** → State와 Instruction 설계
3. **Phase 2: State 정의** → 데이터 구조가 명확해야 Instruction을 설계할 수 있음
4. **Phase 3: Instruction 정의** → 기본 Instructions 작성
5. **Phase 4: Account Validation 정의** → 각 Instruction의 Account 구조체 정의
6. **Phase 5: Handler Instruction 정의** (필요한 경우) → Base Layer Handler 구현
7. **Phase 6: 테스트 및 검증** → 모든 플로우 검증

### ER 사이클 중심 개발 순서

1. **State 정의** → State 구조체 정의
2. **Initialize Instruction** → Base Layer에서 State 생성
3. **Delegate Instruction** → ER에 위임 (ER 사이클 시작)
4. **Operations Instructions** → ER에서 State 조작 (빠르고 저렴)
5. **Commit Instruction** → ER → Base Layer 동기화
   - 단순 commit vs commit + action vs commit + undelegate 결정
6. **Handler Instruction** (필요한 경우) → Base Layer에서 실행될 로직
7. **Undelegate Instruction** (필요한 경우) → ER에서 완전히 복귀
8. **전체 사이클 테스트** → Delegate → Operations → Commit → Undelegate 플로우 검증

---

## ⚠️ 주의사항

1. **Space 계산**: `8 (discriminator) + 각 필드 크기` 정확히 계산
2. **PDA Seeds**: 고유하고 의미있는 seeds 사용
3. **Account 순서**: Handler의 Account 순서가 `ShortAccountMeta`와 일치해야 함
4. **Compute Units**: Handler의 복잡도에 맞게 설정
5. **Escrow Index**: `ActionArgs`의 `escrow_index`가 올바른지 확인
6. **Account Writable**: Handler에서 쓰기 필요하면 `is_writable: true` 설정

---

## 🔄 예시 플로우

### 플로우 1: 기본 ER 사이클 (Delegate → Operations → Commit → Undelegate)
```
[Base Layer]
User → Initialize → State 생성 (Base Layer)

[Base Layer → ER]
User → Delegate → Account를 ER에 위임
  ↓
[ER]
User → Update (on ER) → State 조작 (빠르고 저렴)
User → Update (on ER) → State 조작 (빠르고 저렴)
  ↓
[ER → Base Layer]
User → Commit → Base Layer로 동기화 (Delegate 상태 유지)
또는
User → Commit + Undelegate → Base Layer로 동기화 + 위임 해제
  ↓
[Base Layer]
User → Update (on Base Layer) → Base Layer에서 작업
```

### 플로우 2: Commit with Action (Handler 실행)
```
[ER]
User → CommitWithAction → ER에서 commit + Base Layer handler 실행
  ↓
[Base Layer]
Handler → Committed account 읽기 → Base Layer state 업데이트
  ↓
[ER 또는 Base Layer]
State가 Base Layer에 동기화됨
```

### 플로우 3: 자동 Commit (DelegateConfig 설정)
```
[Base Layer]
User → Delegate → Account를 ER에 위임
  (DelegateConfig: commit_frequency_ms = 30_000)
  ↓
[ER]
User → Update (on ER) → State 조작
  ↓
[자동 Commit - 30초마다]
System → Auto Commit → Base Layer로 자동 동기화
  ↓
[ER]
User → Update (on ER) → 계속 작업 가능
  ↓
[자동 Commit - 30초마다]
System → Auto Commit → Base Layer로 자동 동기화
```

### 플로우 4: 완전한 사이클 (초기화부터 종료까지)
```
1. [Base Layer] Initialize → State 생성
2. [Base Layer] Delegate → ER에 위임
3. [ER] Operations → 여러 번 State 조작
4. [ER] Commit → Base Layer로 동기화 (Delegate 상태 유지)
5. [ER] Operations → 계속 작업
6. [ER] Commit + Undelegate → 동기화 + 위임 해제
7. [Base Layer] Operations → Base Layer에서 최종 작업
```

---

## 📚 참고 패턴

현재 코드베이스의 패턴:

### State 예시
- `Counter`: 기본 State 예시 (count: u64)
- `Leaderboard`: Base Layer State 예시 (high_score: u64)

### Instruction 패턴
- `initialize`: Initialize 패턴 (Base Layer에서 State 생성)
- `increment`: Update 패턴 (ER 또는 Base Layer에서 실행)
- `update_leaderboard`: Handler 패턴 (Base Layer에서 실행, ER에서 commit된 데이터 읽기)
- `delegate`: Delegate 패턴 (Base Layer → ER 위임)
- `undelegate`: Undelegate 패턴 (ER → Base Layer 복귀, commit과 함께)
- `commit_and_update_leaderboard`: Magic Action 패턴 (ER에서 commit + Base Layer handler 실행)

### ER 사이클 패턴
1. **Delegate**: `delegate()` 함수 + `#[delegate]` 매크로 + `delegate_pda()` 메서드
2. **Operations**: ER에서 일반 instructions 실행 (ER에서 실행되지만 코드는 동일)
3. **Commit**: `commit_and_update_leaderboard()` 함수 + `#[commit]` 매크로 + `MagicInstructionBuilder`
4. **Undelegate**: `commit_and_undelegate_accounts()` 함수 또는 `MagicAction::CommitAndUndelegate`

