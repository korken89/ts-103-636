//! Software AES-128 [`MacCrypto`] backend using RustCrypto's `aes`,
//! `cmac`, and `ctr` crates. Available when the `software-crypto`
//! feature is enabled (it is on by default).

use aes::Aes128;
use cmac::Mac;

use super::{KEY_LEN, MacCrypto};

/// Software AES-128 implementation of [`MacCrypto`].
///
/// Suitable as a default for development and for environments where no
/// hardware crypto accelerator is wired up. For production targets with
/// a CryptoCell or similar, implement [`MacCrypto`] against that engine
/// and disable the `software-crypto` feature to strip the RustCrypto
/// dependencies entirely.
#[derive(Debug, Clone, Copy, Default)]
pub struct SoftwareCrypto;

impl MacCrypto for SoftwareCrypto {
    type Error = core::convert::Infallible;

    fn cmac(&mut self, key: &[u8; KEY_LEN], data: &[u8]) -> Result<[u8; 16], Self::Error> {
        let mut mac = <cmac::Cmac<Aes128> as cmac::Mac>::new_from_slice(key)
            .expect("AES-128 key length is fixed");
        Mac::update(&mut mac, data);
        let result = Mac::finalize(mac).into_bytes();
        let mut out = [0; 16];
        out.copy_from_slice(result.as_slice());
        Ok(out)
    }

    fn ctr_apply(
        &mut self,
        key: &[u8; KEY_LEN],
        iv: &[u8; 16],
        data: &mut [u8],
    ) -> Result<(), Self::Error> {
        use aes::cipher::{KeyIvInit, StreamCipher, generic_array::GenericArray};
        type Ctr = ctr::Ctr128BE<Aes128>;
        let mut cipher = Ctr::new(GenericArray::from_slice(key), GenericArray::from_slice(iv));
        cipher.apply_keystream(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{compute_mic, verify_mic};

    #[test]
    fn software_cmac_known_answer_aes128() {
        // NIST SP 800-38B Appendix D.1 K and Tlen=128, empty input.
        let key: [u8; 16] = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let expected: [u8; 16] = [
            0xbb, 0x1d, 0x69, 0x29, 0xe9, 0x59, 0x37, 0x28, 0x7f, 0xa3, 0x7d, 0x12, 0x9b, 0x75,
            0x67, 0x46,
        ];
        let mut crypto = SoftwareCrypto;
        let tag = crypto.cmac(&key, &[]).unwrap();
        assert_eq!(tag, expected);
    }

    #[test]
    fn software_compute_then_verify_mic_round_trips() {
        let mut crypto = SoftwareCrypto;
        let key = [0x42u8; 16];
        let mut buf = [0; 32];
        for (i, b) in buf[..27].iter_mut().enumerate() {
            *b = i as u8;
        }
        compute_mic(&mut crypto, &key, &mut buf).unwrap();
        assert!(verify_mic(&mut crypto, &key, &buf).unwrap());
        buf[0] ^= 1;
        assert!(!verify_mic(&mut crypto, &key, &buf).unwrap());
    }

    #[test]
    fn software_ctr_round_trip() {
        let mut crypto = SoftwareCrypto;
        let key = [0x11u8; 16];
        let iv = [0x22u8; 16];
        let original = [0xAAu8; 33];
        let mut buf = original;
        crypto.ctr_apply(&key, &iv, &mut buf).unwrap();
        assert_ne!(buf, original);
        crypto.ctr_apply(&key, &iv, &mut buf).unwrap();
        assert_eq!(buf, original);
    }
}
