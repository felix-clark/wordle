#!/usr/bin/env python
import argparse
from collections import Counter
from typing import List, Tuple, Optional
from tqdm import tqdm

import numpy as np


WORD_SIZE: int = 5


def get_feedback(secret: str, guess: str) -> str:
    """
    Return feedback of a guess versus a secret.

    - : no match
    + : right letter, wrong location
    * : right letter in correct location
    """
    assert len(secret) == len(guess)
    # match_loc = [a == b for a, b in zip(secret, guess)]
    # result = ['*' if m else '-' for m in match_loc]
    result = ['-'] * WORD_SIZE
    secret_ctr = Counter(secret)
    guess_ctr = Counter(guess)
    # This holds the lowest number of each letter
    common_ctr = secret_ctr & guess_ctr
    # NOTE: right now the copy is unnecessary (and possibly will always be)
    wrong_loc_ctr = common_ctr.copy()
    # remove location matches from common counter
    for i, (a, b) in enumerate(zip(secret, guess)):
        if a == b:
            result[i] = '*'
            wrong_loc_ctr[a] -= 1
    for lett, count in wrong_loc_ctr.items():
        idxs = [i for i, lg in enumerate(guess)
                if lg == lett and result[i] != '*']
        for i in idxs[:count]:
            assert result[i]
            result[i] = '+'
    return ''.join(result)


def letter_pool(guess: str, feedback: str) -> Counter:
    assert not (set(feedback) - set('-+*'))
    ctr = Counter([
        lg for lg, fb in zip(guess, feedback) if fb != '-'
    ])
    return ctr


def reduce_dict(dictionary: List[str], guess: str, feedback: str) -> List[str]:
    guess_lett_pool = letter_pool(guess, feedback)
    exact_letts: List[Tuple] = [
        (idx, lett) for idx, (lett, fb)
        in enumerate(zip(guess, feedback))
        if fb == "*"
    ]
    wrong_locs: List[Tuple] = [
        (idx, lett) for idx, (lett, fb)
        in enumerate(zip(guess, feedback))
        if fb == "+"
    ]
    # TODO: This will not deal with an upper bound on some letters
    pre_wrong_letts = [lett for lett, fb in zip(guess, feedback) if fb == '-']
    # The list of letters that are present but we know are limited
    limits = {k: guess_lett_pool[k]
              for k in guess_lett_pool.keys() & pre_wrong_letts}
    # This is imprecise because it doesn't deal with duplicate letters properly
    wrong_letts = pre_wrong_letts & guess_lett_pool.keys()
    wrong_letts = {
        lett for lett in pre_wrong_letts
        if lett not in guess_lett_pool
    }
    # NOTE: We shouldn't need to remove the exact matches from the letter pools
    # since they will be subtracted out the same.
    slimmed: List[str] = []
    for word in dictionary:
        # Check for exact letter matches
        if any((
            word[idx] != lett
            for idx, lett in exact_letts
        )):
            continue
        word_ctr = Counter(word)
        # Ensure no prohibited letters appear
        if word_ctr.keys() & wrong_letts:
            continue
        # Ensure that all matched letters appear
        if guess_lett_pool - word_ctr:
            continue
        if any((
            word[idx] == lett
            for idx, lett in wrong_locs
        )):
            continue
        if any(word_ctr[lett] > lim for lett, lim in limits.items()):
            continue
        slimmed.append(word)
    return slimmed


def get_expected_remain_after(
    dictionary: List[str],
    guess: str,
) -> float:
    n_lefts: List[int] = []
    for word in dictionary:
        feedback = get_feedback(word, guess)
        reduced_dict = reduce_dict(dictionary, guess, feedback)
        n_left = len(reduced_dict)
        n_lefts.append(n_left)
    return np.mean(n_lefts)


def get_letter_loc_entropy(
    dictionary: List[str],
    guess: str,
) -> float:
    """Entropy from letters in specific positions"""
    # NOTE: This may not really be an "entropy"
    dict_size = len(dictionary)
    assert len(guess) == WORD_SIZE
    lett_counts = [0] * WORD_SIZE
    for word in dictionary:
        for i in range(WORD_SIZE):
            if word[i] == guess[i]:
                lett_counts[i] += 1
    lett_freqs = [c/dict_size for c in lett_counts]
    # The 2nd term accounts for the letter being incorrect
    ent = -sum([xlogx(x) + xlogx(1. - x) for x in lett_freqs])
    return ent


def xlogx(x: float) -> float:
    return x * np.log(x) if x != 0. else 0.


def get_letter_dist_entropy(
    dictionary: List[str],
    guess: str,
) -> float:
    """KL divergence from letter count distribution"""
    dict_len = len(dictionary)
    assert len(guess) == WORD_SIZE
    guess_ctr = Counter(guess)
    guess_letts = guess_ctr.keys()
    dict_ctrs = {l: Counter() for l in guess_ctr.keys()}
    for word in dictionary:
        word_ctr = Counter([lett for lett in word if lett in guess_letts])
        for k, v in word_ctr.items():
            dict_ctrs[k].update([v])
    for k, ctr in dict_ctrs.items():
        normed_ctr = {0: dict_len - sum(ctr.values())}
        normed_ctr.update(ctr)
        normed_ctr = {l: m / dict_len for l, m in normed_ctr.items()}
        dict_ctrs[k] = normed_ctr
    # TODO: When evaluating against the entire word list, the dictionary's distribution
    # should be pre-calculated for all letters.
    ent = 0.
    for l, c in guess_ctr.items():
        # l_p = c / len(guess)
        try:
            ps = [dict_ctrs[l][cp] for cp in range(c)]
        except:
            print(guess)
            print(l, c)
            print(dict_ctrs[l])
            raise
        ps.append(1. - sum(ps))
        ent -= sum([xlogx(p) for p in ps])
    return ent


def get_letter_dist_kl(
    dictionary: List[str],
    guess: str,
) -> float:
    """KL divergence from letter count distribution"""
    dict_len = len(dictionary)
    assert len(guess) == WORD_SIZE
    guess_ctr = Counter(guess)
    guess_letts = guess_ctr.keys()
    dict_ctrs = {l: Counter() for l in guess_ctr.keys()}
    for word in dictionary:
        word_ctr = Counter([lett for lett in word if lett in guess_letts])
        for k, v in word_ctr.items():
            dict_ctrs[k].update([v])
    # kl = - sum([c / len(guess) * np.log(c/dict_ctrs[l]) for l, c in guess_ctr.items()])
    kl = 0.
    for l, c in guess_ctr.items():
        l_p = c / len(guess)
        lett_ctr = dict_ctrs[l]
        # lett_ctr_tot = lett_ctr.total() # python 3.10
        # lett_ctr_tot = sum(lett_ctr.values())
        # Normalize by word length to account for zeros
        lett_ctr_tot = dict_len
        l_q = lett_ctr[c] / lett_ctr_tot
        kl -= l_p * np.log(l_p / l_q)
    return kl


def get_max_entropy(
    dictionary: List[str],
    testwords: Optional[List[str]] = None,
):
    if testwords is None:
        testwords = dictionary
    # maxent = 0.
    maxent = -10.
    bestword = None
    for word in testwords:
        # wordent = get_letter_loc_entropy(dictionary, word)
        # wordent = get_letter_dist_kl(dictionary, word)
        # This is the best combo that uses KL, although we'd think you'd want
        # to minimize KLD
        wordent = get_letter_loc_entropy(
            dictionary, word
            # )
            # ) + get_letter_dist_kl(dictionary, word)
            ) + get_letter_dist_entropy(dictionary, word)

        if wordent == maxent:
            print(bestword, word)
        if wordent > maxent:
            bestword = word
            maxent = wordent
    return bestword, maxent


def get_solution_list() -> List[str]:
    with open("./dict/wordle_solutions.txt", 'r') as fin:
        return [line.strip().upper() for line in fin]


def get_full_dict() -> List[str]:
    with open("./dict/wordle_complete_dictionary.txt", 'r') as fin:
        return [line.strip().upper() for line in fin]


def live_repl():
    solutions = get_solution_list()
    # allowed_guesses = get_full_dict()
    allowed_guesses = get_solution_list()
    while True:
        if len(solutions) > 128:
            nextguess, _ = get_max_entropy(solutions, allowed_guesses)
        else:
            nextguess = None
            best_rem = 256
            for w in solutions:
                n_exp_after = get_expected_remain_after(solutions, w)
                if n_exp_after < best_rem:
                    nextguess = w
                    best_rem = n_exp_after

        print(f"Recommended guess: {nextguess}")
        guess = input("Enter guess (blank for recommended): ").strip().upper()
        if not guess:
            guess = nextguess
        assert len(guess) == WORD_SIZE
        feedback = input("Enter feedback: ").strip()
        assert len(feedback) == WORD_SIZE
        assert not (set(list(feedback)) - set(list('-+*')))
        solutions = reduce_dict(solutions, guess, feedback)
        n_solutions = len(solutions)
        if not n_solutions:
            print("No valid solutions:")
            break
        if n_solutions == 1:
            print(f"Solution: {solutions[0]}")
            break
        print(f"{n_solutions} possible solutions remain")


def main():
    parser = argparse.ArgumentParser(description="Wordle solver")
    parser.add_argument("command", type=str, choices=["test", "live"])
    parser.add_argument("--word-size", type=int, default=WORD_SIZE)
    args = parser.parse_args()

    if args.command == "live":
        live_repl()
        exit(0)

    assert args.command == "test"
    assert args.word_size == WORD_SIZE

    secret = "OCEAN"
    guess = "KAZOO"
    # secret = "PRICK"
    # guess = "CRIMP"
    print(secret)
    print(guess)
    feedback = get_feedback(secret, guess)
    print(feedback)

    solution_dict = get_solution_list()
    n_solutions = len(solution_dict)
    init_ent = np.log(n_solutions)
    print(n_solutions)
    print(init_ent)
    reduced_dict = reduce_dict(solution_dict, guess, feedback)
    n_after = len(reduced_dict)
    print(n_after)
    print(reduced_dict[:24])

    guesses = [
        "RAISE",
        "RILES",
        "SLATE",
        "IRATE",
        "ROAST",
        "NOTES",
        "RESIN",
        "TARES",
        "SENOR",
        "SAINT",
        "WHINY",
        "MAMMA",
        "TATTY",
    ]

    print(get_max_entropy(solution_dict))
    for guess in guesses:
        avg_left: float = get_expected_remain_after(solution_dict, guess)
        exact_ent: float = np.log(avg_left)
        ent_loc = get_letter_loc_entropy(solution_dict, guess)
        ent_dist = get_letter_dist_entropy(solution_dict, guess)
        ent_approx = ent_loc + ent_dist
        print(f"{guess}:\t{avg_left:.2f}\t{init_ent-exact_ent:.2f}\t{ent_approx:.2f}\t{ent_loc:.2f}\t{ent_dist:.2f}")
        # print(get_letter_dist_kl(solution_dict, guess))


if __name__ == "__main__":
    main()
