# Navidrome Smart Plugin

Analyze listening activity of the user and build personalized mixes
with [AudioMuse AI](https://github.com/NeptuneHub/AudioMuse-AI).

## Requirements

- Navidrome
- AudioMuse AI

## Algorithm

### My Vibe

TODO: come up with a better solution.

Current implementation is too simple and gives too random songs.

I simply take the last 15 songs, guess the average mood and seed it
to alchemy. There should be smarter ways.

### Daily Mixes

1. Build a weighted listening profile

Fetch listening history from the last 6 months.

For each song calculate a preference weight:

w = ln(1 + sum(e^(-decay_rate * age)))

Use a half-life of 45 days:

decay_rate = ln(2) / 45

2. Cluster the songs

Fetch embedding for each song and build 6 clusters using K-Means algorithm.

3. Find similar tracks

Filter out unwanted tracks from the clusters.

For each cluster:

1. Take N = min(top_songs, 30) songs (sorted by preference weight)
2. Run Alchemy on these songs to get similar tracks
3. Apply filters for similar tracks
4. Take 50 - N similar tracks and add them to the mixture.
5. Save playlist as Daily Mix #N

## Roadmap

- [ ] My Vibe
- [ ] Daily Mixes
- [ ] Your Artist Mixes
