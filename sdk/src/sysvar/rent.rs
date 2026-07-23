use solana_program_error::ProgramError;

/// Account storage overhead for calculation of base rent.
///
/// This is the number of bytes required to store an account with no data. It is
/// added to an accounts data length when calculating [`Rent::try_minimum_balance`].
pub const ACCOUNT_STORAGE_OVERHEAD: u64 = 128;

/// Maximum permitted size of account data (10 MiB).
pub const MAX_PERMITTED_DATA_LENGTH: u64 = 10 * 1024 * 1024;

/// Maximum lamports per byte value.
const MAX_LAMPORTS_PER_BYTE: u64 = 1_759_197_129_867;

pub struct Rent {
    /// Rental rate in lamports per byte.
    pub lamports_per_byte: u64,
}

// Assert that the size of the `Rent` struct is as expected (8 bytes).
const _ASSERT_STRUCT_LEN: () = assert!(size_of::<Rent>() == 8);

// Assert that the alignment of the `Rent` struct is as expected (8 byte).
const _ASSERT_STRUCT_ALIGN: () = assert!(align_of::<Rent>() == 8);

impl Rent {
    /// Calculates the minimum balance for rent exemption without performing
    /// any validation.
    ///
    /// # Important
    ///
    /// The caller must ensure that `data_len` is within the permitted limit
    /// and the `lamports_per_byte` is within the permitted limit based on
    /// the `exemption_threshold` to avoid overflow.
    ///
    /// # Arguments
    ///
    /// * `data_len` - The number of bytes in the account
    ///
    /// # Returns
    ///
    /// The minimum balance in lamports for rent exemption.
    #[inline(always)]
    pub fn minimum_balance_unchecked(&self, data_len: usize) -> u64 {
        (ACCOUNT_STORAGE_OVERHEAD + data_len as u64) * self.lamports_per_byte
    }

    /// Calculates the minimum balance for rent exemption.
    ///
    /// This method avoids floating-point operations when the
    /// `exemption_threshold` is the default value.
    ///
    /// # Arguments
    ///
    /// * `data_len` - The number of bytes in the account
    ///
    /// # Returns
    ///
    /// The minimum balance in lamports for rent exemption.
    ///
    /// # Errors
    ///
    /// Returns `ProgramError::InvalidArgument` if `data_len` exceeds the
    /// maximum permitted data length or if the `lamports_per_byte` is too
    /// large based on the `exemption_threshold`, which would cause an
    /// overflow.
    #[inline(always)]
    pub fn try_minimum_balance(&self, data_len: usize) -> Result<u64, ProgramError> {
        if data_len as u64 > MAX_PERMITTED_DATA_LENGTH {
            return Err(ProgramError::InvalidArgument);
        }

        // Validate `lamports_per_byte` based on `exemption_threshold` to prevent
        // overflow.

        if self.lamports_per_byte > MAX_LAMPORTS_PER_BYTE {
            return Err(ProgramError::InvalidArgument);
        }

        Ok(self.minimum_balance_unchecked(data_len))
    }

    /// Determines if an account can be considered rent exempt.
    ///
    /// # Arguments
    ///
    /// * `lamports` - The balance of the account in lamports
    /// * `data_len` - The size of the account in bytes
    ///
    /// # Returns
    ///
    /// `true`` if the account is rent exempt, `false`` otherwise.
    #[allow(deprecated)]
    #[inline]
    pub fn is_exempt(&self, lamports: u64, data_len: usize) -> bool {
        lamports >= self.minimum_balance_unchecked(data_len)
    }
}

impl_get!(Rent, 5);
