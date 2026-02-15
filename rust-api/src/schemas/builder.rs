use crate::constants::*;
use crate::error::Result;
use super::types::*;

/// Builder for GenerateParams with method chaining and validation on build.
pub struct GenerateParamsBuilder {
    params: GenerateParams,
}

impl GenerateParamsBuilder {
    /// Create a new builder with the given prompt. All other fields use defaults.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            params: GenerateParams {
                prompt: prompt.into(),
                ..Default::default()
            },
        }
    }

    // -- Non-Option scalar setters -------------------------------------------

    /// Set the generation action (Generate, Img2Img, Infill).
    /// The data-carrying variants include all action-specific fields.
    pub fn action(mut self, action: GenerateAction) -> Self {
        self.params.action = action;
        self
    }

    pub fn model(mut self, model: Model) -> Self {
        self.params.model = model;
        self
    }

    pub fn width(mut self, width: u32) -> Self {
        self.params.width = width;
        self
    }

    pub fn height(mut self, height: u32) -> Self {
        self.params.height = height;
        self
    }

    pub fn steps(mut self, steps: u32) -> Self {
        self.params.steps = steps;
        self
    }

    pub fn scale(mut self, scale: f64) -> Self {
        self.params.scale = scale;
        self
    }

    pub fn cfg_rescale(mut self, cfg_rescale: f64) -> Self {
        self.params.cfg_rescale = cfg_rescale;
        self
    }

    pub fn sampler(mut self, sampler: Sampler) -> Self {
        self.params.sampler = sampler;
        self
    }

    pub fn noise_schedule(mut self, noise_schedule: NoiseSchedule) -> Self {
        self.params.noise_schedule = noise_schedule;
        self
    }

    // -- Option<u64> setter --------------------------------------------------

    pub fn seed(mut self, seed: u64) -> Self {
        self.params.seed = Some(seed);
        self
    }

    // -- Option<Vec<T>> setters ----------------------------------------------

    pub fn characters(mut self, characters: Vec<CharacterConfig>) -> Self {
        self.params.characters = Some(characters);
        self
    }

    /// Set vibes using `Vec<VibeConfig>`, which bundles each vibe item
    /// together with its strength and info_extracted values.
    pub fn vibes(mut self, vibes: Vec<VibeConfig>) -> Self {
        self.params.vibes = Some(vibes);
        self
    }

    // -- Option<struct> setter -----------------------------------------------

    pub fn character_reference(mut self, char_ref: CharacterReferenceConfig) -> Self {
        self.params.character_reference = Some(char_ref);
        self
    }

    // -- Option<String> setters ----------------------------------------------

    pub fn negative_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.params.negative_prompt = Some(prompt.into());
        self
    }

    // -- SaveTarget setters --------------------------------------------------

    /// Set save target to an exact file path.
    pub fn save_path(mut self, path: impl Into<String>) -> Self {
        self.params.save = SaveTarget::ExactPath(path.into());
        self
    }

    /// Set save target to a directory (with optional filename via `SaveTarget::Directory`).
    pub fn save_dir(mut self, dir: impl Into<String>) -> Self {
        self.params.save = SaveTarget::Directory { dir: dir.into(), filename: None };
        self
    }

    // -- Build ---------------------------------------------------------------

    /// Build and validate the params. Returns error if validation fails.
    pub fn build(self) -> Result<GenerateParams> {
        self.params.validate()?;
        Ok(self.params)
    }
}

/// Convenience method on GenerateParams
impl GenerateParams {
    pub fn builder(prompt: impl Into<String>) -> GenerateParamsBuilder {
        GenerateParamsBuilder::new(prompt)
    }
}
