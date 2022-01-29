# wordle
Solve Wordle puzzles in the fewest possible steps

## Solving a puzzle

Run the following command:
```cargo run --release solve```
from within the project directory. A guess will be recommended (the first one may take some
time). Press `return` to use the guess or enter your own. You will be prompted for feedback,
which will accept the alphabet of "-+*" for wrong letters, misplaced letters, and correct
letters, respectively.

For instance, if you choose the guess of "RIVER" and receive a yellow 'V' and a green 'E', the
feedback you enter should be "--+*-".
