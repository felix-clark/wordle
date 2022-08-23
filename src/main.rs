use anyhow::anyhow;
use clap::Parser;
use itertools::{all, any, Itertools};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

mod counter;
use counter::Counter;
mod letter_dist;
use letter_dist::{LettCountDist, LettLocDist};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Mode of operation
    #[clap(takes_value = true, possible_values = ["test", "solve", "play"])]
    prog: String,
    /// Initial word guess
    #[clap(long, takes_value = true)]
    first_guess: Option<String>,
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

fn read_feedback<const M: usize>(s: &str) -> anyhow::Result<Feedback<M>> {
    let result: Vec<LettFb> = s
        .chars()
        .map(|c| match c {
            '-' => Ok(LettFb::Grey),
            '+' => Ok(LettFb::Yellow),
            '*' => Ok(LettFb::Green),
            _ => Err(anyhow!("Invalid feedback string {s}")),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let fb: Feedback<M> = result.as_slice().try_into()?;
    Ok(fb)
}

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
        // NOTE: Clippy warns about this "unnecessary collect()" but it really is needed because
        // we check result in the first pass and mutate it in the second.
        // To avoid this the indices with greens could be pre-computed to skip.
        let idxs: Vec<usize> = guess
            .iter()
            .enumerate()
            .filter(|(i, &lg)| (lg == lett) && !matches!(result[*i], LettFb::Green))
            .map(|(i, _)| i)
            .collect();
        for i in idxs.into_iter().take(count) {
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
                correct_lett_ctr.add(lett);
            }
            LettFb::Green => {
                exact_letts.push((idx, lett));
                correct_lett_ctr.add(lett);
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
    let lett_limits: BTreeMap<u8, usize> = correct_lett_ctr
        .iter()
        .filter(|(k, _)| marked_wrong_letts.contains(k))
        .map(|(&k, &v)| (k, v))
        .collect();

    let result: Vec<Word<5>> = dict
        .par_iter()
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
    let reader = BufReader::new(f);
    let words: Vec<Word<5>> = reader
        .lines()
        .map(|l| Ok(l?.to_ascii_uppercase().as_bytes().try_into()?))
        .collect::<anyhow::Result<_>>()?;
    Ok(words)
}

fn get_extra_dict() -> anyhow::Result<Vec<Word<5>>> {
    // TODO: Make relative path so it can be ran from any directory
    let f = File::open("dict/wordle_complete_dictionary.txt")?;
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
    let sum_remain = n_remain.into_iter().map(|u| u as f32).sum::<f32>();
    // subtract 1 if the word is in the dictionary to prefer possible correct answers
    norm * if dict.iter().any(|w| w == guess) {
        sum_remain - 1.
    } else {
        sum_remain
    }
}

fn get_best_expect(dict: &[Word<5>], pool: &[Word<5>]) -> (Word<5>, f32) {
    let exp_lefts: Vec<f32> = pool
        .par_iter()
        .map(|w| get_expect_remain_after(dict, w))
        .collect();
    let (exp_left, best_guess) = exp_lefts
        .iter()
        .zip(pool.iter())
        .min_by(|(elx, _), (ely, _)| elx.partial_cmp(ely).unwrap())
        .unwrap();
    (*best_guess, *exp_left)
}

fn filter_top_heur(dict: &[Word<5>], pool: &[Word<5>], n: usize) -> Vec<Word<5>> {
    let lett_cnt_dist = LettCountDist::new(dict);
    let lett_loc_dist = LettLocDist::new(dict);
    let lett_cnt_ents: Vec<f32> = pool.par_iter().map(|w| lett_cnt_dist.entropy(w)).collect();
    let lett_loc_ents: Vec<f32> = pool.par_iter().map(|w| lett_loc_dist.entropy(w)).collect();
    let total_ents: Vec<f32> = lett_cnt_ents
        .iter()
        .zip_eq(lett_loc_ents.iter())
        .map(|(a, b)| a + b)
        .collect();
    // The solution pool has to be queried specifically because an actual solution can be drowned
    // out in the large dictionary
    let lett_cnt_ents_dict: Vec<f32> = dict.par_iter().map(|w| lett_cnt_dist.entropy(w)).collect();
    let lett_loc_ents_dict: Vec<f32> = dict.par_iter().map(|w| lett_loc_dist.entropy(w)).collect();
    let total_ents_dict: Vec<f32> = lett_cnt_ents_dict
        .iter()
        .zip_eq(lett_loc_ents_dict.iter())
        .map(|(a, b)| a + b)
        .collect();

    let mut total_ents_dict_sort = total_ents_dict.clone();
    let mut total_ents_sort = total_ents.clone();

    // TODO: We don't need to sort the whole list, we should be able to get the top n
    total_ents_sort.sort_unstable_by(|a, b| {
        a.partial_cmp(b)
            .unwrap_or_else(|| panic!("could not compare {a}, {b}"))
    });
    total_ents_dict_sort.sort_unstable_by(|a, b| {
        a.partial_cmp(b)
            .unwrap_or_else(|| panic!("could not compare {a}, {b}"))
    });

    let idx_max = if n > total_ents_sort.len() {
        0
    } else {
        total_ents_sort.len() - n
    };
    let ent_cutoff = total_ents_sort[idx_max];
    let idx_max = if n > total_ents_dict_sort.len() {
        0
    } else {
        total_ents_dict_sort.len() - n
    };
    let ent_cutoff_dict = total_ents_dict_sort[idx_max];

    let pass_pool = total_ents
        .into_iter()
        .zip_eq(pool.iter())
        .filter_map(|(s, w)| if s >= ent_cutoff { Some(*w) } else { None });
    let pass_dict = total_ents_dict
        .into_iter()
        .zip_eq(dict.iter())
        .filter_map(|(s, w)| if s >= ent_cutoff_dict { Some(*w) } else { None });
    pass_pool
        .chain(pass_dict)
        .collect::<HashSet<Word<5>>>()
        .into_iter()
        .collect_vec()
}

fn word_to_string<const M: usize>(w: Word<M>) -> String {
    String::from_utf8(w.to_vec()).expect("Invalid UTF8")
}

fn run_solve_repl(init: Option<String>) -> anyhow::Result<()> {
    let sol_dict = get_dictionary()?;
    let full_dict: Vec<Word<5>> = sol_dict
        .iter()
        .to_owned()
        .chain(get_extra_dict()?.iter())
        .cloned()
        .collect();

    let mut guess_hist: Vec<(Word<5>, Feedback<5>)> = Vec::new();
    let mut avail_solutions = sol_dict;
    let mut line_buf = String::new();

    if let Some(first_guess) = init {
        let first_guess = first_guess.to_ascii_uppercase();
        println!("Input feedback for {first_guess}:");
        line_buf.drain(..);
        let _bin = std::io::stdin()
            .read_line(&mut line_buf)
            .expect("Could not read stdin");
        let feedback = read_feedback::<5>(line_buf.trim())?;
        let first_guess: Word<5> = first_guess.as_bytes().try_into()?;
        avail_solutions = reduce_dict(&avail_solutions, &first_guess, &feedback);
        let n_remain = avail_solutions.len();
        println!("{n_remain} solutions left");
        guess_hist.push((first_guess, feedback));
    }
    while avail_solutions.len() > 1 {
        let filtered_by_heur = filter_top_heur(&avail_solutions, &full_dict, 24);
        // let n_filtered = filtered_by_heur.len();
        // println!("{n_filtered} filtered");
        let (best_guess, exp_n) = get_best_expect(&avail_solutions, &filtered_by_heur);
        let best_guess_str = word_to_string(best_guess);
        println!("Best guess: {best_guess_str} ({exp_n:.2})");
        println!("Input guess (leave blank for recommended):");
        line_buf.drain(..);
        let _bin = std::io::stdin()
            .read_line(&mut line_buf)
            .expect("Could not read stdin");
        let trimmed = line_buf.trim();
        let guess: Word<5> = if trimmed.is_empty() {
            best_guess
        } else {
            trimmed.to_ascii_uppercase().as_bytes().try_into()?
        };
        let guess_str = word_to_string(guess);
        println!("Input feedback for {guess_str}:");
        line_buf.drain(..);
        let _bin = std::io::stdin()
            .read_line(&mut line_buf)
            .expect("Could not read stdin");
        let feedback = read_feedback::<5>(line_buf.trim())?;
        avail_solutions = reduce_dict(&avail_solutions, &guess, &feedback);
        let n_remain = avail_solutions.len();
        println!("{n_remain} solutions left");
        if n_remain < 8 && n_remain > 1 {
            let words: String = avail_solutions
                .iter()
                .cloned()
                .map(word_to_string)
                .intersperse("\t".to_string())
                .collect();
            println!("{words}");
        }
        guess_hist.push((guess, feedback));
    }
    if avail_solutions.is_empty() {
        println!("No solutions found!");
        let mut alt_solutions = full_dict;
        for (gw, fb) in guess_hist {
            alt_solutions = reduce_dict(&alt_solutions, &gw, &fb);
        }
        println!("Possible extended options:");
        let alt_solutions_str: String = alt_solutions
            .into_iter()
            .map(word_to_string)
            .intersperse(" ".to_string())
            .collect();
        println!("{alt_solutions_str}");
        return Err(anyhow!("No solutions found!"));
    } else {
        let solution = word_to_string(avail_solutions[0]);
        println!("The solution is {solution}");
    }
    Ok(())
}

fn run_test() -> anyhow::Result<()> {
    let sol_dict = get_dictionary()?;
    let n_dict = sol_dict.len();
    let init_ent = (n_dict as f32).ln();
    println!("{n_dict}");
    println!("Hello, world!");
    let secret: Word<5> = "WINCE".as_bytes().try_into()?;
    let guess: Word<5> = "SLATE".as_bytes().try_into()?;
    let feedback = get_feedback(&secret, &guess);
    let r1 = reduce_dict(&sol_dict, &guess, &feedback);
    let n_red = r1.len();
    println!("{n_red}");

    let lett_cnt_dist = LettCountDist::new(&sol_dict);
    let lett_loc_dist = LettLocDist::new(&sol_dict);

    // RAISE and ARISE both have 168 worst-case remaining.
    // Raise is slightly better on average: 61 vs. ARISE's 63.7.
    let tests = ["RAISE", "ARISE", "ROATE", "SLATE", "SAINT", "RESIN"];
    for g in tests {
        let gw = g.as_bytes().try_into()?;
        let exp_left = get_expect_remain_after(&sol_dict, gw);
        let ent_exact = init_ent - exp_left.ln();
        let ent_cnt = lett_cnt_dist.entropy(gw);
        let ent_loc = lett_loc_dist.entropy(gw);
        // let ent_total = ent_cnt + ent_loc;
        println!("{g}:\t{exp_left:.2}\t{ent_exact:.2}\t{ent_cnt:.2}\t{ent_loc:.2}");
    }

    let filtered = filter_top_heur(&sol_dict, &sol_dict, 24);
    let filtered_strings = filtered
        .iter()
        .map(|w| std::str::from_utf8(w).unwrap())
        .collect_vec();
    println!("{filtered_strings:?}");

    // let (best_guess, approx_ent) = get_best_expect_heur(&sol_dict, &filtered);
    let (best_guess, approx_ent) = get_best_expect(&sol_dict, &filtered);
    let best_guess: String = String::from_utf8(best_guess.to_vec())?;
    println!("{best_guess}:\t{approx_ent:.2}");

    // let (best_guess, exp_left) = get_best_expect(&sol_dict, &sol_dict);
    // let best_guess: String = String::from_utf8(best_guess.to_vec())?;
    // println!("{best_guess}:\t{exp_left:.2}");

    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.prog.as_str() {
        "test" => {
            run_test()?;
        }
        "solve" => {
            run_solve_repl(args.first_guess)?;
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
