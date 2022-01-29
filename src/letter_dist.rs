//! Letter distribution functions
use std::{collections::BTreeMap, iter};

use itertools::Itertools;

use crate::counter::Counter;
use crate::Word;

pub(crate) struct LettCountDist<const M: usize> {
    lett_cts: BTreeMap<u8, BTreeMap<usize, usize>>,
    dict_size: usize,
}

impl<const M: usize> LettCountDist<M> {
    pub(crate) fn new(dict: &[Word<M>]) -> Self {
        let dict_size: usize = dict.len();
        // The key of the top-level map is the letter. The value is another map whose key is
        // the number of counts in a word, and the value is the number of times that count
        // appears.
        // e.g. ["PEARS", "APPLE"] => {A:{1: 2}, P: {1: 1, 2: 1}, ...}
        // If a letter does not appear in a word then no count is added, so afterwards these
        // counts must be infered with the dict size.
        let mut lett_cts = BTreeMap::<u8, BTreeMap<usize, usize>>::new();
        for word in dict {
            let word_ctr: Counter = word.iter().cloned().collect();
            // TODO: don't use an explicit for loop here
            for (&lett, &count) in word_ctr.iter() {
                *lett_cts
                    .entry(lett)
                    .or_insert_with(BTreeMap::new)
                    .entry(count)
                    .or_insert(0) += 1;
            }
        }
        for ct_ctr in lett_cts.values_mut() {
            let sum_cts = ct_ctr.values().sum::<usize>();
            ct_ctr.insert(0, dict_size - sum_cts);
        }
        Self {
            lett_cts,
            dict_size,
        }
    }

    pub(crate) fn entropy(&self, word: &Word<M>) -> f32 {
        let norm: f32 = 1. / self.dict_size as f32;
        let word_ctr: Counter = word.iter().cloned().collect();
        // The response can determine the exact letter count if the dictionary word has fewer
        // instances of a given letter than the guess does.
        // If a dictionary word has as many or more instances of a letter relative to a guess word,
        // we only know it has at least that many.
        -self
            .lett_cts
            .iter()
            .map(|(l, l_freq)| {
                let l_ct = word_ctr.get(l);
                let ns: Vec<usize> = l_freq
                    .iter()
                    .filter_map(
                        |(lett_count, freq)| {
                            if lett_count < l_ct {
                                Some(*freq)
                            } else {
                                None
                            }
                        },
                    )
                    .collect();
                let n_rem = self.dict_size - ns.iter().sum::<usize>();
                let ps: Vec<f32> = ns
                    .into_iter()
                    .chain(iter::once(n_rem))
                    .map(|n| n as f32 * norm)
                    .collect();
                ps.into_iter().map(xlnx).sum::<f32>()
            })
            .sum::<f32>()
    }
}

pub(crate) struct LettLocDist<const M: usize> {
    counts: [BTreeMap<u8, f32>; M],
}

impl<const M: usize> LettLocDist<M> {
    pub(crate) fn new(dict: &[Word<M>]) -> Self {
        // Const initialization of an array using a function is not yet stabilized. See:
        // https://github.com/rust-lang/rust/issues/89379
        let mut counters: Vec<Counter> = vec![Counter::new(); M];
        for word in dict {
            for (&lett, counter) in word.iter().zip_eq(counters.iter_mut()) {
                counter.add(lett);
            }
        }
        let counts: Vec<BTreeMap<_, _>> =
            counters.into_iter().map(|ctr| ctr.normalized()).collect();
        let counts = counts.try_into().unwrap();
        Self { counts }
    }

    pub(crate) fn entropy(&self, word: &Word<M>) -> f32 {
        -word
            .iter()
            .zip_eq(self.counts.iter())
            .map(|(l, cts)| {
                let p: f32 = cts.get(l).cloned().unwrap_or(0.);
                xlnx(p) + xlnx(1. - p)
            })
            .sum::<f32>()
    }
}

fn xlnx(x: f32) -> f32 {
    if x != 0. {
        x * x.ln()
    } else {
        0.
    }
}
