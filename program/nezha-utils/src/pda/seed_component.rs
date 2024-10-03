use solana_program::pubkey::Pubkey;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SeedComponent {
    Str(&'static str),
    Pubkey(Pubkey),
    U64([u8; 8]),
    U32([u8; 4]),
    U8([u8; 1]),
}

impl Default for SeedComponent {
    fn default() -> Self {
        SeedComponent::U8([0])
    }
}

impl AsRef<[u8]> for SeedComponent {
    fn as_ref(&self) -> &[u8] {
        match self {
            SeedComponent::Str(s) => s.as_ref(),
            SeedComponent::Pubkey(s) => s.as_ref(),
            SeedComponent::U64(x) => x,
            SeedComponent::U32(x) => x,
            SeedComponent::U8(x) => x,
        }
    }
}

impl From<&'static str> for SeedComponent {
    fn from(s: &'static str) -> Self {
        SeedComponent::Str(s)
    }
}

impl From<Pubkey> for SeedComponent {
    fn from(s: Pubkey) -> Self {
        SeedComponent::Pubkey(s)
    }
}

impl From<u64> for SeedComponent {
    fn from(s: u64) -> Self {
        SeedComponent::U64(s.to_le_bytes())
    }
}

impl From<u32> for SeedComponent {
    fn from(s: u32) -> Self {
        SeedComponent::U32(s.to_le_bytes())
    }
}

impl From<u8> for SeedComponent {
    fn from(s: u8) -> Self {
        SeedComponent::U8(s.to_le_bytes())
    }
}
