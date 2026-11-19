# Navidrome Smart Plugin

Analyze listening activity of the user and build smart recommendations
with [AudioMuse AI](https://github.com/NeptuneHub/AudioMuse-AI).

## Requirements

- ListenBrainz (will be optional when native [Navidrome API](https://www.navidrome.org/docs/usage/features/scrobbling/#scrobble-history) gets stable enough)
- AudioMuse AI

## Architecture

General overview

TODO (rewrite)

Analyze listening activity of the user. Determine the songs that
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

- Skip unwanted songs like film soundtracks
- Consider user inactivity as a factor?
