# My Vibe

## Architecture

### Collect Statistics

#### Determine the most obsessed songs

1. Fetch listens from the last 3-5 days 
2. Calculate the score for each song using a formula:

Song Score = sum(1 / (hours_elapsed_since_listen + 12))

3. Sort resulting songs by their score in descending order to determine the most
obsessed songs. Take the first 3-5 songs from the results.

#### Determine the most obsessed artist

Group songs by their artist and calculate the score:

Artist Score = sum(song_score) * ln(unique_songs + 1)

#### Define general mood

TODO

## Unsolved problems:

- Mood mismatch
- Skip unwanted songs like film soundtracks
