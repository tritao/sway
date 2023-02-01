use crate::Value;

#[derive(Debug, Clone)]
pub enum EVMInstruction {
    /// Halts execution
    Stop,
    /// Addition operation
    Add,
    /// Multiplication operation
    Mul,
    /// Subtraction operation
    Sub,
    /// Integer division operation
    Div,
    /// Signed integer division operation (truncated)
    SDiv,
    /// Modulo remainder operation
    Mod,
    /// Signed modulo remainder operation
    SMod,
    /// Modulo addition operation
    AddMod,
    /// Modulo multiplication operation
    MulMod,
    /// Exponential operation
    Exp,
    /// Extend length of two’s complement signed integer
    SignExtend,
    /// Less-than comparison
    Lt,
    /// Greater-than comparison
    Gt,
    /// Signed less-than comparison
    SLt,
    /// Signed greater-than comparison
    SGt,
    /// Equality comparison
    Eq,
    /// Zero comparison
    IsZero,
    /// Bitwise AND operation
    And,
    /// Bitwise OR operation
    Or,
    /// Bitwise XOR operation
    Xor,
    /// Bitwise NOT operation
    Not,
    /// Retrieve single byte from word
    Byte,
    /// Left shift operation
    Shl,
    /// Right shift operation
    Shr,
    /// Arithmetic (signed) right shift operation
    Sar,
    /// Compute Keccak-256 hash
    SHA3,
    /// Get address of currently executing account
    Address,
    /// Get balance of the given account
    Balance,
    /// Get execution origination address
    Origin,
    /// Get caller address
    Caller,
    /// Get deposited value by the instruction/transaction responsible for this execution
    CallValue,
    /// Get input data of current environment
    CallDataLoad,
    /// Get size of input data in current environment
    CallDataSize,
    /// Copy input data in current environment to memory
    CallDataCopy,
    /// Get size of code running in current environment
    CodeSize,
    /// Copy code running in current environment to memory
    CodeCopy,
    /// Get price of gas in current environment
    GasPrice,
    /// Get size of an account’s code
    ExtCodeSize,
    /// Copy an account’s code to memory
    ExtCodeCopy,
    /// Get size of output data from the previous call from the current environment
    ReturnDataSize,
    /// Copy output data from the previous call to memory
    ReturnDataCopy,
    /// Get hash of an account’s code
    ExtCodeHash,
    /// Get the hash of one of the 256 most recent complete blocks
    BlockHash,
    /// Get the block’s beneficiary address
    Coinbase,
    /// Get the block’s timestamp
    Timestamp,
    /// Get the block’s number
    Number,
    /// Get the previous block’s RANDAO mix
    PrevRANDAO,
    /// Get the block’s gas limit
    GasLimit,
    /// Get the chain ID
    ChainId,
    /// Get balance of currently executing account
    SelfBalance,
    /// Get the base fee
    BaseFee,
    /// Remove item from stack
    Pop,
    /// Load word from memory
    MLoad,
    /// Save word to memory
    MStore,
    /// Save byte to memory
    MStore8,
    /// Load word from storage
    SLoad,
    /// Save word to storage
    SStore,
    /// Alter the program counter
    Jump,
    /// Conditionally alter the program counter
    JumpI,
    /// Get the value of the program counter prior to the increment corresponding to this instruction
    PC,
    /// Get the size of active memory in bytes
    MSize,
    /// Get the amount of available gas, including the corresponding reduction for the cost of this instruction
    Gas,
    /// Mark a valid destination for jumps
    JumpDest,
    /// Place n (up to 32) byte items on stack
    Push { size: usize, value: Value },
    /// Duplicate nth (1 up to 16) stack item
    Dup { index: usize },
    /// Exchange 1st and nth+1 (1 up to 16) stack items
    Swap { size: usize },
    /// Append log record with n (0 up to 4) topics
    Log { topics: usize },
    /// Create a new account with associated code
    Create,
    /// Message-call into an account
    Call,
    /// Message-call into this account with alternative account’s code
    CallCode,
    /// Halt execution returning output data
    Return,
    /// Message-call into this account with an alternative account’s code, but persisting the current values for sender and value
    DelegateCall,
    /// Create a new account with associated code at a predictable address
    Create2,
    /// Static message-call into an account
    StaticCall,
    /// Halt execution reverting state changes but returning data and remaining gas
    Revert,
    /// Designated invalid instruction
    Invalid,
    /// Halt execution and register account for later deletion
    SelfDestruct,
}
