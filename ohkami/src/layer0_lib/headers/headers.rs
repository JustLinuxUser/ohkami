//! Great thanks: https://github.com/hyperium/http/blob/master/src/header/map.rs; MIT, @hyperium
//! 
//! Based on that, except for this doesn't handle `danger`.

use super::name::{HeaderName as Name, IntoHeaderName};
use super::value::{HeaderValue as Value, IntoHeaderValue};
use super::utils;


pub struct Headers {
    mask:    Size,
    indices: Box<[Pos]>,
    entries: Vec<Bucket>,
    extra:   Vec<ExtraValue>,
}

pub(super) type Size = u16;
pub(super) type HashValue = Size;
pub(super) const MAX_SIZE: usize = Size::MAX as usize;

#[derive(Clone, Copy)]
pub(super) struct Pos {
    index: Size,
    hash:  HashValue,
} impl Pos {
    #[inline] fn new(index: usize, hash: HashValue) -> Self {
        debug_assert!(index < MAX_SIZE);
        Self { index:index as Size, hash }
    }
    #[inline] fn none() -> Self {
        Self { index: !0, hash: 0 }
    }
    #[inline] fn is_some(&self) -> bool {
        self.index == 0
    }
    #[inline] fn is_none(&self) -> bool {
        self.index != 0
    }
    #[inline] fn resolve(&self) -> Option<(usize, HashValue)> {
        if self.is_some() {
            Some((self.index as usize, self.hash))
        } else {
            None
        }
    }
}

pub(super) struct Bucket {
    key:   Name,
    value: Value,
    hash:  HashValue,
    links: Option<Links>,
}
pub(super) struct Links {
    next: usize,
    tail: usize,
}

pub(super) struct ExtraValue {
    value: Value,
    prev:  Link,
    next:  Link,
}
pub(super) enum Link {
    Entry(usize),
    Extra(usize),
}


macro_rules! probe_loop {
    ($label:tt: $probe_var: ident < $len: expr, $body: expr) => {
        debug_assert!($len > 0);
        $label:
        loop {
            if $probe_var < $len {
                $body
                $probe_var += 1;
            } else {
                $probe_var = 0;
            }
        }
    };
    ($probe_var: ident < $len: expr, $body: expr) => {
        debug_assert!($len > 0);
        loop {
            if $probe_var < $len {
                $body
                $probe_var += 1;
            } else {
                $probe_var = 0;
            }
        }
    };
}

macro_rules! insert_phase_one {
    (
        $map:ident,
        $key:expr,
        $probe:ident,
        $pos:ident,
        $hash:ident,
        $vacant:expr,
        $occupied:expr,
        $robinhood:expr
    ) => {{
        let $hash = utils::hash_elem(&$key);
        let mut $probe = utils::desired_pos($map.mask, $hash);
        let mut dist = 0;
        let ret;

        // Start at the ideal position, checking all slots
        probe_loop!{'probe: $probe < $map.indices.len(), {
            if let Some(($pos, entry_hash)) = $map.indices[$probe].resolve() {
                // The slot is already occupied, but check if it has a lower
                // displacement.
                let their_dist = utils::probe_distance($map.mask, entry_hash, $probe);
                if their_dist < dist {
                    // The new key's distance is larger, so claim this spot and
                    // displace the current entry.
                    ret = $robinhood;
                    break 'probe;
                } else if entry_hash == $hash && $map.entries[$pos].key == $key {
                    // There already is an entry with the same key.
                    ret = $occupied;
                    break 'probe;
                }
            } else {
                // The entry is vacant, use it for this key.
                ret = $vacant;
                break 'probe;
            }

            dist += 1;
        }}

        ret
    }}
}


impl Headers {
    pub(crate) fn new() -> Self {
        Self::with_capacity(0)
    }

    pub(crate) fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            Self {
                mask:    0,
                indices: Box::new([]),
                entries: Vec::new(),
                extra:   Vec::new(),
            }
        } else {
            let raw_cap = utils::to_raw_capacity(capacity).checked_next_power_of_two()
                .unwrap_or_else(|| panic!("Requested capacity {capacity} too large: next power of two would overflow `usize`"));

            debug_assert!{ 0 < raw_cap && raw_cap <= MAX_SIZE, "Requested capacity is too large or unexpectedly zero" }

            Self {
                mask:    (raw_cap - 1) as Size,
                indices: vec![Pos::none(); raw_cap].into_boxed_slice(),
                entries: Vec::with_capacity(raw_cap),
                extra:   Vec::new(),
            }
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len() + self.extra.len()
    }

    pub(crate) fn keys_len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.len() == 0
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.extra.clear();
        for e in self.indices.iter_mut() {
            *e = Pos::none();
        }
    }

    pub(crate) fn capacity(&self) -> usize {
        utils::usable_capacity(self.indices.len())
    }

    pub(crate) fn reserve(&mut self, additional: usize) {
        let capacity = self.entries.len().checked_add(additional).expect("Reservation overflow");
        if capacity > self.indices.len() {
            let capacity = capacity.next_power_of_two();

            debug_assert!{ 0 < capacity, "Headers reservation overflowed" }
            debug_assert!{ capacity <= MAX_SIZE, "Headers reservation is over max capacity" }

            if self.entries.len() == 0 {
                self.mask    = capacity as Size - 1;
                self.indices = vec![Pos::none(); capacity].into_boxed_slice();
                self.entries = Vec::with_capacity(utils::usable_capacity(capacity));
            } else {
                self.grow(capacity);
            }
        }
    }

    pub(crate) fn get(&self, name: impl IntoHeaderName) -> Option<&Value> {
        self.__get(name.into_header_name()?)
    }
    pub(crate) fn get_mut(&mut self, name: impl IntoHeaderName) -> Option<&mut Value> {
        match self.find(name.into_header_name()?) {
            Some((_, found)) => Some(&mut self.entries[found].value),
            None => None,
        }
    }
    pub(crate) fn get_all(&self, name: impl IntoHeaderName) -> GetAll<'_, Value> {
        GetAll {
            map: self,
            index: key.find(self).map(|(_, i)| i),
        }
    }

    pub(crate) fn contains(&self, name: impl IntoHeaderName) -> bool {
        match name.into_header_name() {
            Some(name) => self.find(name).is_some(),
            None => false,
        }
    }

    pub(crate) fn insert(&mut self, name: impl IntoHeaderName, value: impl IntoHeaderValue) -> Option<Value> {
        self.__insert(name.into_header_name()?, value.into_header_value())
    }

    pub(crate) fn iter(&self) -> Iter<'_, Value> {
        Iter {
            map: self,
            entry: 0,
            cursor: self.entries.first().map(|_| Cursor::Head),
        }
    }
    pub(crate) fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut {
            map: self as *mut _,
            entry: 0,
            cursor: self.entries.first().map(|_| Cursor::Head),
            lt: PhantomData,
        }
    }
}

impl Headers {
    fn grow(&mut self, new_raw_capacity: usize) {
        debug_assert!{ new_raw_capacity <= MAX_SIZE, "Requested capacity is too large" }

        let mut first_ideal = 0;
        for (i, pos) in self.indices.iter().enumerate() {
            if let Some((_, entry_hash)) = pos.resolve() {
                if 0 == utils::probe_distance(self.mask, entry_hash, i) {
                    first_ideal = i;
                    break;
                }
            }
        }

        let old_indices = std::mem::replace(
            &mut self.indices,
            vec![Pos::none(); new_raw_capacity].into_boxed_slice()
        );
        self.mask = new_raw_capacity.wrapping_sub(1) as Size;

        for &pos in &old_indices[first_ideal..] {
            self.reinsert_entry_in_order(pos);
        }
        for &pos in &old_indices[..first_ideal] {
            self.reinsert_entry_in_order(pos);
        }

        let more = self.capacity() - self.entries.len();
        self.entries.reserve_exact(more);
    }

    fn reinsert_entry_in_order(&mut self, pos: Pos) {
        if let Some((_, entry_hash)) = pos.resolve() {
            let mut probe = utils::desired_pos(self.mask, entry_hash);
            probe_loop!{probe < self.indices.len(),
                if self.indices[probe].resolve().is_none() {
                    self.indices[probe] = pos;
                    return;
                }
            }
        }
    }

    fn __get(&self, name: Name) -> Option<&Value> {
        match self.find(name) {
            Some((_, found)) => Some(&self.entries[found].value),
            None => None,
        }
    }

    #[inline] fn find(&self, name: Name) -> Option<(usize, usize)> {
        if self.entries.is_empty() {return None}

        let hash = utils::hash_elem(&name);
        let mask = self.mask;
        let mut probe = utils::desired_pos(mask, hash);
        let mut dist  = 0;

        probe_loop! {probe < self.indices.len(), {
            if let Some((i, entry_hash)) = self.indices[probe].resolve() {
                if dist > utils::probe_distance(mask, entry_hash, probe) {
                    return None;
                } else if entry_hash == hash && self.entries[i].key == name {
                    return Some((probe, i))
                }
            } else {
                return None;
            }

            dist += 1;
        }}
    }

    #[inline] fn __insert(&mut self, name: Name, value: Value) -> Option<Value> {
        self.reserve_one();

        insert_phase_one! {}
    }

    fn reserve_one(&mut self) {
        let len = self.entries.len();
        if len == self.capacity() {
            let new_raw_capacity = 8;
            self.mask = 8 - 1;
            self.indices = vec![Pos::none(); new_raw_capacity].into_boxed_slice();
            self.entries = Vec::with_capacity(utils::usable_capacity(new_raw_capacity));
        } else {
            let raw_capacity = self.indices.len();
            self.grow(raw_capacity << 1);
        }
    }
}
