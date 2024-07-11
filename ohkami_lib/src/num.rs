#[inline]
pub fn hexized(n: usize) -> String {
    unsafe {String::from_utf8_unchecked(
        hexized_bytes(n).into()
    )}
}

#[inline(always)]
pub fn hexized_bytes(n: usize) -> [u8; std::mem::size_of::<usize>() * 2] {
    use std::mem::{size_of, transmute};

    unsafe {
        transmute::<_, [u8; size_of::<usize>() * 2]>(
            n.to_be_bytes().map(|byte| [byte>>4, byte&0b1111])
        ).map(|h| h + match h {
            0..=9   => b'0'-0,
            10..=15 => b'a'-10,
            _ => std::hint::unreachable_unchecked()
        })
    }
}

#[cfg(test)]
#[test] fn test_hexize() {
    for (n, expected) in [
        (1,   "1"),
        (9,   "9"),
        (12,  "c"),
        (16,  "10"),
        (42,  "2a"),
        (314, "13a"),
    ] {
        assert_eq!(hexized(n).trim_start_matches('0'), expected)
    }
}


#[inline]
pub fn itoa(mut n: usize) -> String {
    const MAX: usize = usize::ilog10(usize::MAX) as _;

    #[cfg(target_pointer_width = "64")]
    const _/* static assert */: [(); 19] = [(); MAX];
    
    let mut buf = Vec::<u8>::with_capacity(1 + MAX);

    {
        let mut push_unchecked = |byte| {
            let len = buf.len();
            unsafe {
                std::ptr::write(buf.as_mut_ptr().add(len), byte);
                buf.set_len(len + 1);
            }
        };

        macro_rules! unroll {
            () => {};
            ($digit:expr) => {unroll!($digit,)};
            ($digit:expr, $($tail:tt)*) => {
                if $digit <= MAX && n >= 10_usize.pow($digit) {
                    unroll!($($tail)*);
                    let q = n / 10_usize.pow($digit);
                    push_unchecked(b'0' + q as u8);
                    n -= 10_usize.pow($digit) * q
                }
            };
        }

        unroll!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19);

        push_unchecked(b'0' + n as u8);
    }
    
    unsafe {String::from_utf8_unchecked(buf)}
}

/// SAFETY: `buf` has enough remaining capacity for UTF-8 representation of `n`
#[inline]
pub unsafe fn encode_itoa_unchecked(n: usize, buf: &mut Vec<u8>) {
    let head = buf.as_mut_ptr();
    let mut p = head.add(buf.len());
    raw_u64(&mut p, n as u64);
    buf.set_len(p.offset_from(head) as usize);

    /* =============================================================== */

    macro_rules! invariant {
        ($expr: expr) => {
            #[cfg(debug_assertions)] {
                assert!($expr);
            }
            if !($expr) {
                unsafe {core::hint::unreachable_unchecked()}
            }
        };
    }
    const D4: [u32; 10000] = {
        let (mut d4, mut i) = ([0u32; 10000], 0u32);
        while i < 10000 {
            let (dh, dl) = (i / 100, i % 100);
            d4[i as usize] = ((dl % 10) << 24) | ((dl / 10) << 16) | ((dh % 10) << 8) | (dh / 10) | 0x30303030;
            i += 1;
        }
        d4
    };
    #[inline(always)]
    unsafe fn raw_d4l(v: &mut *mut u8, x: u32) {
        invariant!(x < 10000);
        match x {
            0..=9 => {
                **v = b'0' + x as u8;
                *v = v.add(1);
            }
            10..=99 => {
                v.copy_from_nonoverlapping((D4[x as usize] >> 16).to_le_bytes().as_ptr(), 2);
                *v = v.add(2);
            }
            100..=999 => {
                v.copy_from_nonoverlapping((D4[x as usize] >> 8).to_le_bytes().as_ptr(), 3);
                *v = v.add(3);
            }
            1000..=9999 => {
                v.copy_from_nonoverlapping(D4[x as usize].to_le_bytes().as_ptr(), 4);
                *v = v.add(4);
            }
            _ => core::hint::unreachable_unchecked(),
        }
    }
    #[inline(always)]
    unsafe fn raw_d4(v: &mut *mut u8, x: u32) {
        invariant!(x < 1_0000);
        v.copy_from_nonoverlapping(D4[x as usize].to_le_bytes().as_ptr(), 4);
        *v = v.add(4);
    }
    #[inline(always)]
    unsafe fn raw_d8l(v: &mut *mut u8, x: u32) {
        invariant!(x < 1_0000_0000);
        if x < 10000 {
            raw_d4l(v, x);
        } else {
            let (y0, y1) = (x / 1_0000, x % 1_0000);
            raw_d4l(v, y0);
            raw_d4(v, y1);
        }
    }
    #[inline(always)]
    unsafe fn raw_d8(v: &mut *mut u8, x: u32) {
        invariant!(x < 1_0000_0000);
        let (y0, y1) = (x / 1_0000, x % 1_0000);
        v.copy_from_nonoverlapping((((D4[y1 as usize] as u64) << 32) | (D4[y0 as usize] as u64)).to_le_bytes().as_ptr(), 8);
        *v = v.add(8);
    }
    #[inline(always)]
    pub unsafe fn raw_u64(v: &mut *mut u8, x: u64) {
        match x {
            0..=9999_9999 => {
                raw_d8l(v, x as u32);
            }
            1_0000_0000..=9999_9999_9999_9999 => {
                let (z0, z1) = ((x / 1_0000_0000) as u32, (x % 1_0000_0000) as u32);
                raw_d8l(v, z0);
                raw_d8(v, z1);
            }
            1_0000_0000_0000_0000..=u64::MAX => {
                let (y0, y1) = (
                    (x / 1_0000_0000_0000_0000) as u32,
                    x % 1_0000_0000_0000_0000,
                );
                let (z0, z1) = ((y1 / 1_0000_0000) as u32, (y1 % 1_0000_0000) as u32);
                raw_d8l(v, y0);
                raw_d8(v, z0);
                raw_d8(v, z1);
            }
        }
    }
}

#[cfg(test)]
mod test_itoa {
    use super::{encode_itoa_unchecked, itoa};

    #[test] fn test_itoa() {
    for n in [
        0,
        1,
        4,
        10,
        11,
        99,
        100,
        109,
        999,
        1000,
        10_usize.pow(usize::ilog10(usize::MAX)) - 1,
        10_usize.pow(usize::ilog10(usize::MAX)),
        usize::MAX - 1,
        usize::MAX,
        ] {
            assert_eq!(itoa(n), n.to_string());
            assert_eq!({
                let mut buf = String::with_capacity(1 + usize::MAX.ilog10() as usize);
                unsafe {encode_itoa_unchecked(n, buf.as_mut_vec())}
                buf
            }, n.to_string());
        }
    }
}
