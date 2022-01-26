use anyhow::anyhow as err;
use std::collections::{BTreeMap, BTreeSet, btree_map};
use std::{cmp, ops};

/// Analogous to python's collections.Counter, specialized for this task
#[derive(Debug)]
pub(crate) struct Counter {
    inner: BTreeMap<u8, u8>,
}

impl FromIterator<u8> for Counter {
    fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
        let mut inner = BTreeMap::new();
        for l in iter {
            *inner.entry(l).or_insert(0) += 1;
        }
        Self { inner }
    }
}

impl Counter {
    pub fn new() -> Self {
        Self {
            inner: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, k: u8) {
        *self.inner.entry(k).or_insert(0) += 1;
    }

    pub fn contains_key(&self, key: &u8) -> bool {
        self.inner.contains_key(key)
    }

    pub fn get(&self, key: &u8) -> &u8 {
        self.inner.get(key).unwrap_or(&0)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.values().sum::<u8>() == 0
    }

    pub fn iter(&self) -> btree_map::Iter<u8, u8> {
        self.inner.iter()
    }

    pub fn keys(&self) -> btree_map::Keys<u8, u8> {
        self.inner.keys()
    }

    pub fn normalized(&self) -> BTreeMap<u8, f32> {
        let total: f32 = self.inner.values().sum::<u8>() as f32;
        self.inner
            .iter()
            .map(|(&k, v)| (k, *v as f32 / total))
            .collect()
    }

    pub fn pop_one(&mut self, k: &u8) -> anyhow::Result<()> {
        let val: &mut u8 = self.inner.get_mut(k).ok_or(err!(""))?;
        if *val < 1 {
            return Err(err!("Counter is already empty at {k}"));
        }
        *val -= 1;
        Ok(())
    }

}

impl ops::BitAnd for Counter {
    type Output = Counter;

    fn bitand(self, rhs: Self) -> Self::Output {
        // TODO: Could possibly optimize by not creating separate keys; just iterate over self's
        // and look up rhs's.
        let self_keys: BTreeSet<&u8> = self.inner.keys().collect();
        let rhs_keys: BTreeSet<&u8> = rhs.inner.keys().collect();
        let common_keys = self_keys.bitand(&rhs_keys);
        let inner = common_keys
            .into_iter()
            .map(|&k| {
                let v: u8 = cmp::min(*self.inner.get(&k).unwrap(), *rhs.inner.get(&k).unwrap());
                (k, v)
            })
            .collect();
        Counter { inner }
    }
}

impl ops::Sub for &Counter {
    type Output = Counter;

    fn sub(self, rhs: Self) -> Self::Output {
        let inner: BTreeMap<u8, u8> = self.inner.iter().filter_map(|(&k, &v)| {
            match rhs.inner.get(&k) {
                Some(&rv) => {
                    v.checked_sub(rv).map(|d| (k, d))
                }
                None => Some((k, v))
            }
        }).collect();
        Counter { inner }
    }
}

impl IntoIterator for Counter {
    type Item = <BTreeMap<u8, u8> as IntoIterator>::Item;
    type IntoIter = <BTreeMap<u8, u8> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
