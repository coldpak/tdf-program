# 🎯 Trading Derivatives Framework (TDF) Program

A comprehensive Solana program built with Anchor that implements a trading derivatives framework with league competitions, position management, and privacy features using MagicBlock's Ephemeral Rollups SDK and x402 protocol.

## 📋 Overview

The TDF Program is a decentralized trading platform that enables:
- **Market Management**: Create and manage trading markets with price feeds
- **League Competitions**: Organize trading competitions with leaderboards
- **Position Trading**: Open, manage, and close leveraged trading positions
- **Privacy Features**: Private resource management using x402 protocol and Private Ephemeral Rollups
- **Magic Actions**: Automatic on-chain handlers when committing accounts from Ephemeral Rollups to the base layer

## 🏗️ Architecture

### Core Components

- **Global Config**: Admin-controlled configuration (fees, treasury, admin)
- **Markets**: Trading pairs with price feeds (Pyth integration), leverage limits
- **Leagues**: Trading competitions with entry fees, rewards, and time windows
- **Participants**: User accounts within leagues tracking equity, PnL, volume
- **Positions**: Leveraged trading positions with real-time PnL calculation via MagicBlock ER
- **Leaderboards**: 
  - On-chain leaderboard with Top-K rankings by equity and trading volume
  - Pay-to-Reveal functionality for private position data
- **Private Resources**: Privacy-enabled resources using MagicBlock's permission system and x402 protocol

### Technology Stack

- **Solana**: Base blockchain layer
- **Anchor**: Solana framework for program development
- **Ephemeral Rollups SDK**: Privacy and off-chain computation
- **Pyth Network**: Price oracle integration
- **MagicBlock Permission System**: Access control for private resources
- **x402 Protocol**: Payment-gated information access protocol for pay-to-reveal functionality

## 📦 Software Requirements

| Software   | Version | Installation Guide                                              |
| ---------- | ------- | --------------------------------------------------------------- |
| **Solana** | 2.1.0+  | [Install Solana](https://docs.anza.xyz/cli/install)             |
| **Rust**   | 1.82+   | [Install Rust](https://www.rust-lang.org/tools/install)         |
| **Anchor** | 0.31.1  | [Install Anchor](https://www.anchor-lang.com/docs/installation) |
| **Node**   | 22.17.0+ | [Install Node](https://nodejs.org/en/download/current)          |

### Setup Instructions

```sh
# Check and initialize your Solana version
agave-install list
agave-install init 2.1.21

# Check and initialize your Rust version
rustup show
rustup install 1.82

# Check and initialize your Anchor version
avm list
avm use 0.31.1

# Install Node dependencies
yarn install
```

## 🚀 Getting Started

### Build the Program

```bash
anchor build
```

### Deploy to Devnet

```bash
# Deploy the program
anchor deploy
```

### Run Tests

```bash
# Run tests with existing deployed program
anchor test --skip-deploy --skip-build --skip-local-validator

# Build, deploy and run all tests
anchor test
```

## 📚 Program Structure

```
programs/tdf-program/src/
├── lib.rs                 # Main program entry point
├── state.rs               # Account structures (GlobalConfig, Market, League, Position, etc.)
├── errors.rs              # Custom error codes
├── constants.rs           # Program constants
├── utils.rs               # Utility functions (price fetching, calculations)
└── instructions/
    ├── initialize.rs      # Program initialization
    ├── market.rs          # Market management (create, update, delete)
    ├── league/
    │   ├── create_league.rs
    │   ├── start_league.rs
    │   ├── close_league.rs
    │   ├── join_league.rs
    │   └── update_leaderboard.rs
    ├── position/
    │   ├── open_position.rs
    │   ├── close_position.rs
    │   ├── commit_position.rs
    │   └── process_participant.rs
    └── private/
        └── example.rs      # Private resource examples
```

## 🔧 Key Features

1. **Leveraged Trading**: Positions support configurable leverage up to market max
2. **Real-time PnL**: Realtime position updating via MagicBlock Ephemeral Rollups
3. **Competition System**: Leagues with entry fees, virtual balances, and rewards
4. **On-chain Leaderboard**: Dual rankings by equity and trading volume, fully on-chain
5. **Pay-to-Reveal**: Monetize private position data with customizable payment requirements
6. **Privacy**: Private position and resource management with x402 protocol integration

## 🔐 Privacy & Pay-to-Reveal

### Current Implementation

The program integrates with MagicBlock's Ephemeral Rollups for privacy:
- **Delegation**: Accounts can be delegated to ephemeral rollups for private computation
- **Commit**: Private state can be committed back to the base layer
- **Permissions**: Fine-grained access control using MagicBlock's permission system
- **Private Resources**: Example implementation of private resource management

### Current Limitations

**Private Position & Pay-to-Reveal Position**

The current implementation of private positions and pay-to-reveal position functionality is limited by the constraints of Ephemeral Rollups:

- **Realtime Price Feed Access**: Ephemeral Rollups cannot directly access realtime price feeds from oracles like Pyth Network. This limitation prevents the program from calculating real-time PnL and executing position management logic within the private rollup environment.
- **Alternative Approach Needed**: To enable fully on-chain private position management with realtime price feeds, a different architecture is required that allows seamless integration between private computation and public oracle data.

## 🧪 Testing

Tests are located in `tests/tdf_program.ts` and cover:

- Program initialization
- Market creation and management
- League lifecycle (create, start, close)
- Position operations (open, close, commit)
- Participant management
- Leaderboard updates
- Private resource operations

Run tests with:
```bash
yarn test
# or
anchor test
```

## 📝 Development Notes

### Important Constraints

- Maximum 10 markets per league
- Maximum 10 positions per participant
- Top-K leaderboard limited to 10 participants
- Position sequence tracking for state management

## 🤝 Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📄 License

ISC
