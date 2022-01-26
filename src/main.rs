use clap::Parser;
use itertools::{all, any};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

mod counter;
use counter::Counter;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(takes_value = true, possible_values = ["test", "solve", "play"])]
    prog: String,
}

type Word<const M: usize> = [u8; M];

/// Feedback on a letter can come in three forms
#[derive(Clone, Copy, Debug)]
enum LettFb {
    /// Wrong letter
    Grey,
    /// Right letter in wrong location
    Yellow,
    /// Correct letter and location
    Green,
}
type Feedback<const M: usize> = [LettFb; M];

fn get_feedback<const M: usize>(secret: &Word<M>, guess: &Word<M>) -> Feedback<M> {
    let secret_ctr: Counter = secret.iter().cloned().collect();
    let guess_ctr: Counter = guess.iter().cloned().collect();
    let mut common_ctr = secret_ctr & guess_ctr;
    let mut result: Feedback<M> = [LettFb::Grey; M];
    for (i, (a, b)) in secret.iter().zip(guess.iter()).enumerate() {
        if a == b {
            result[i] = LettFb::Green;
            common_ctr
                .pop_one(a)
                .expect("Should have a value to pop here");
        }
    }
    for (lett, count) in common_ctr.into_iter() {
        let idxs: Vec<usize> = guess
            .iter()
            .enumerate()
            .filter(|(i, &lg)| (lg == lett) && !matches!(result[*i], LettFb::Green))
            .map(|(i, _)| i)
            .collect();
        for i in idxs.into_iter().take(count.into()) {
            result[i] = LettFb::Yellow;
        }
    }
    result
}

fn reduce_dict(dict: &[Word<5>], guess: &Word<5>, feedback: &Feedback<5>) -> Vec<Word<5>> {
    // Letters marked correctly, with correct counts, that may or may not be in the proper
    // location.
    // NOTE: We could construct this after the fact with wrong_locs and exact_letts
    let mut correct_lett_ctr = Counter::new();
    // Indices and letters in the exact right location
    let mut exact_letts: Vec<(usize, u8)> = Vec::new();
    // Indices and letters marked incorrectly. In the case of duplicate guess letters, some of
    // these might be present elsewhere in the secret word.
    // TODO: Work out if this needs the indices
    // let mut marked_wrong_letts: Vec<(usize, u8)> = Vec::new();
    let mut marked_wrong_letts: BTreeSet<u8> = BTreeSet::new();
    // Letters marked as present in the wrong location
    let mut wrong_locs: Vec<(usize, u8)> = Vec::new();

    for (idx, (&lett, &fb)) in guess.iter().zip(feedback.iter()).enumerate() {
        match fb {
            // LettFb::Grey => marked_wrong_letts.push((idx, lett)),
            LettFb::Grey => {
                marked_wrong_letts.insert(lett);
            }
            LettFb::Yellow => {
                wrong_locs.push((idx, lett));
                correct_lett_ctr.insert(lett);
            }
            LettFb::Green => {
                exact_letts.push((idx, lett));
                correct_lett_ctr.insert(lett);
            }
        }
    }

    // Letters that aren't in the secret word
    let wrong_letts: BTreeSet<u8> = marked_wrong_letts
        .iter()
        // .map(|(_, l)| l)
        .filter(|l| !correct_lett_ctr.contains_key(l))
        .cloned()
        .collect();
    // Upper bounds on the counts of specific letters. This can come up when a letter is
    // duplicated in the guess but not the secret.
    let lett_limits: BTreeMap<u8, u8> = correct_lett_ctr
        .iter()
        .filter(|(k, _)| marked_wrong_letts.contains(k))
        .map(|(&k, &v)| (k, v))
        .collect();

    let result: Vec<Word<5>> = dict
        .iter()
        .filter(|w| {
            let w_ctr: Counter = w.iter().cloned().collect();
            // Require any exact letter matches
            all(&exact_letts, |(idx, lett)| w[*idx] == *lett) &&
            // Ensure that no prohibited letters appear
            !any(w_ctr.keys(), |k| wrong_letts.contains(k)) &&
            // Ensure that all matched letters appear
            (&correct_lett_ctr - &w_ctr).is_empty() &&
            // Make sure the word doesn't have letters in the wrong locations
            !any(&wrong_locs, |(idx, lett)| w[*idx] == *lett) &&
            // Enforce letter limits
            // TODO: This may be obsolete in view of the (correct_lett_ctr - w_ctr).is_empty()
            // check above.
            all(&lett_limits, |(l, x)| w_ctr.get(l) <= x)
            // Duplicate greyed letters that do exist in the word should be filtered by the
            // combination of the letter counts and the letter limits
        })
        .cloned()
        .collect();
    result
}

fn get_dictionary() -> anyhow::Result<Vec<Word<5>>> {
    // TODO: Make relative path so it can be ran from any directory
    let f = File::open("dict/wordle_solutions.txt")?;
    // let f = File::open("dict/wordle_complete_dictionary.txt")?;
    let reader = BufReader::new(f);

    let words: Vec<Word<5>> = reader
        .lines()
        .map(|l| Ok(l?.to_ascii_uppercase().as_bytes().try_into()?))
        .collect::<anyhow::Result<_>>()?;

    Ok(words)
}

fn get_expect_remain_after(dict: &[Word<5>], guess: &Word<5>) -> f32 {
    let n_remain: Vec<usize> = dict
        .iter()
        .map(|w| {
            let fb = get_feedback(w, guess);
            let reduced = reduce_dict(dict, guess, &fb);
            reduced.len()
        })
        .collect();
    // Worst-case scenario:
    // let max_remain = n_remain.into_iter().max().unwrap();
    // max_remain as f32
    let norm = 1. / n_remain.len() as f32;
    norm * n_remain.into_iter().map(|u| u as f32).sum::<f32>()
}

fn run_solve_repl() -> anyhow::Result<()> {
    todo!()
}

fn run_test() -> anyhow::Result<()> {
    let sol_dict = get_dictionary()?;
    let n_dict = sol_dict.len();
    println!("{n_dict}");
    println!("Hello, world!");
    let secret: Word<5> = "WINCE".as_bytes().try_into()?;
    let guess: Word<5> = "SLATE".as_bytes().try_into()?;
    let feedback = get_feedback(&secret, &guess);
    let r1 = reduce_dict(&sol_dict, &guess, &feedback);
    let n_red = r1.len();
    println!("{n_red}");

    // RAISE and ARISE both have 168 worst-case remaining.
    // Raise is slightly better on average: 61 vs. ARISE's 63.7.
    let tests = ["RAISE", "ARISE", "ROATE", "SLATE", "SAINT", "RESIN"];
    for g in tests {
        let gw = g.as_bytes().try_into()?;
        let exp_left = get_expect_remain_after(&sol_dict, gw);
        println!("{g}:\t{exp_left:.2}");
    }

    let exp_lefts: Vec<f32> = sol_dict
        .iter()
        .map(|w| get_expect_remain_after(&sol_dict, w))
        .collect();
    let (exp_left, best_guess) = exp_lefts
        .iter()
        .zip(sol_dict.iter())
        .min_by(|(elx, _), (ely, _)| elx.partial_cmp(ely).unwrap())
        .unwrap();
    let best_guess: String = String::from_utf8(best_guess.to_vec())?;
    println!("{best_guess}:\t{exp_left:.2}");

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    println!("{args:?}");
    match args.prog.as_str() {
        "test" => {
            run_test()?;
        }
        "solve" => {
            run_solve_repl()?;
        }
        "play" => {
            todo!();
        }
        _ => {
            unreachable!();
        }
    };
    Ok(())
}
