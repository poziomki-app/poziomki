use serde::Serialize;

/// One of the five harm categories predicted by Bielik-Guard-0.1B-v1.1.
/// The discriminant values match the model's `id2label` mapping so the enum
/// doubles as a stable logit index.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    SelfHarm = 0,
    Hate = 1,
    Vulgar = 2,
    Sex = 3,
    Crime = 4,
}

impl Category {
    pub const ALL: [Self; 5] = [
        Self::SelfHarm,
        Self::Hate,
        Self::Vulgar,
        Self::Sex,
        Self::Crime,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SelfHarm => "self-harm",
            Self::Hate => "hate",
            Self::Vulgar => "vulgar",
            Self::Sex => "sex",
            Self::Crime => "crime",
        }
    }
}

/// Per-category sigmoid probabilities in the range `[0, 1]`.
///
/// The model is multi-label: independent probabilities, not a softmax. A single
/// input can score high in multiple categories simultaneously.
#[derive(Copy, Clone, Debug, Serialize)]
pub struct Scores {
    pub self_harm: f32,
    pub hate: f32,
    pub vulgar: f32,
    pub sex: f32,
    pub crime: f32,
}

impl Scores {
    #[must_use]
    pub const fn get(&self, category: Category) -> f32 {
        match category {
            Category::SelfHarm => self.self_harm,
            Category::Hate => self.hate,
            Category::Vulgar => self.vulgar,
            Category::Sex => self.sex,
            Category::Crime => self.crime,
        }
    }

    /// Categories whose score meets or exceeds the corresponding threshold.
    #[must_use]
    pub fn flagged(&self, thresholds: &Thresholds) -> Vec<(Category, f32)> {
        Category::ALL
            .into_iter()
            .filter_map(|c| {
                let score = self.get(c);
                (score >= thresholds.get(c)).then_some((c, score))
            })
            .collect()
    }

    #[must_use]
    pub fn verdict(&self, thresholds: &Thresholds) -> Verdict {
        let block = Category::ALL
            .into_iter()
            .any(|c| self.get(c) >= thresholds.block_for(c));
        if block {
            return Verdict::Block;
        }
        let flag = Category::ALL
            .into_iter()
            .any(|c| self.get(c) >= thresholds.get(c));
        if flag {
            Verdict::Flag
        } else {
            Verdict::Allow
        }
    }
}

/// Per-category decision thresholds.
///
/// `flag` = below it, ignore; at/above, surface for review. `block` = at/above,
/// refuse synchronously (used for bios, and for chat messages in the
/// highest-stakes categories). `block` must be >= `flag` in each category.
#[derive(Copy, Clone, Debug)]
pub struct Thresholds {
    pub self_harm: f32,
    pub hate: f32,
    pub vulgar: f32,
    pub sex: f32,
    pub crime: f32,

    pub self_harm_block: f32,
    pub hate_block: f32,
    pub vulgar_block: f32,
    pub sex_block: f32,
    pub crime_block: f32,
}

impl Thresholds {
    /// For user bios — synchronous gate, favour precision on the publish path.
    pub const BIO: Self = Self {
        self_harm: 0.50,
        hate: 0.70,
        vulgar: 0.85,
        sex: 0.70,
        crime: 0.60,
        self_harm_block: 0.50,
        hate_block: 0.70,
        vulgar_block: 0.85,
        sex_block: 0.70,
        crime_block: 0.60,
    };

    /// For chat messages — deliver first, flag for async review. Only the
    /// highest-stakes categories synchronously block.
    pub const CHAT: Self = Self {
        self_harm: 0.50,
        hate: 0.70,
        vulgar: 0.90,
        sex: 0.80,
        crime: 0.70,
        self_harm_block: 0.90,
        hate_block: 0.95,
        vulgar_block: 1.01,
        sex_block: 0.95,
        crime_block: 0.90,
    };

    #[must_use]
    pub const fn get(&self, category: Category) -> f32 {
        match category {
            Category::SelfHarm => self.self_harm,
            Category::Hate => self.hate,
            Category::Vulgar => self.vulgar,
            Category::Sex => self.sex,
            Category::Crime => self.crime,
        }
    }

    #[must_use]
    pub const fn block_for(&self, category: Category) -> f32 {
        match category {
            Category::SelfHarm => self.self_harm_block,
            Category::Hate => self.hate_block,
            Category::Vulgar => self.vulgar_block,
            Category::Sex => self.sex_block,
            Category::Crime => self.crime_block,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Verdict {
    Allow,
    Flag,
    Block,
}

impl Verdict {
    /// Stable string form for metric labels and structured log fields.
    /// Matches the `serde(rename_all = "kebab-case")` representation so a
    /// Prometheus dashboard and a JSON log line agree on casing.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Flag => "flag",
            Self::Block => "block",
        }
    }
}
