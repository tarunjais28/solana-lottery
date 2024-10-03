/// Convenience macro to let us type
///     seeds!(PREFIX, "FOO", 123u8)
/// instead of
///     &[SeedComponent::Str(PREFIX), SeedComponent::Str("FOO"), SeedComponent::U8(123)]
#[macro_export]
macro_rules! seeds {
    ($($x: expr),+) => {
        &[$($x.into()),+]
    };
}
