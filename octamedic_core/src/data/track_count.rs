/// How many tracks a block has, as OctaMED offers them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrackCount {
    #[default]
    Four,
    Eight,
    Twelve,
    Sixteen,
}

impl TrackCount {
    pub const ALL: [TrackCount; 4] = [
        TrackCount::Four,
        TrackCount::Eight,
        TrackCount::Twelve,
        TrackCount::Sixteen,
    ];

    pub fn tracks(self) -> usize {
        return match self {
            TrackCount::Four => 4,
            TrackCount::Eight => 8,
            TrackCount::Twelve => 12,
            TrackCount::Sixteen => 16,
        };
    }

    /// The fewest that hold `tracks`; `Sixteen` for more than that.
    pub fn holding(tracks: usize) -> Self {
        return Self::ALL
            .into_iter()
            .find(|count| count.tracks() >= tracks)
            .unwrap_or(TrackCount::Sixteen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_round_up_to_a_count() {
        assert_eq!(TrackCount::holding(4), TrackCount::Four);
        assert_eq!(TrackCount::holding(1), TrackCount::Four);
        assert_eq!(TrackCount::holding(9), TrackCount::Twelve);
        assert_eq!(TrackCount::holding(64), TrackCount::Sixteen);
        assert_eq!(TrackCount::Twelve.tracks(), 12);
    }
}
