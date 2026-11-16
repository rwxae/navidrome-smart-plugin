# My Vibe

## Architecture

General overview

Analyze recent listening activity of the user. Determine the songs that
he is most obsessed with and seed them to AudioMuse AI to build a playlist
with the most relevant songs.

### Determine obsession

1. Fetch user listening history from the last 7-14 days.
2. Calculate the score for each song using the formula:

Song Score = sum(1 / (1 + hours_since_listen / 168))

3. Sort resulting songs by their score in descending order to determine the most
obsessed songs. Take the first 3-8 songs from the results.

### Find similar songs

Run /api/alchemy with the collected data:

Playlist = alchemy(obsessed_songs)

### Filter irrelevant music

- by library id (optional)
- by genre (optional)
- by mood (optional)

### Save the playlist

Map each song to Navidrome ID and save a playlist on the server.

## Future considerations:

- Mood mismatch between songs (select the largest group of songs)
- Skip unwanted songs like film soundtracks
- The most recent listens get really high score. I don't like it.
- Consider user inactivity as a factor?
