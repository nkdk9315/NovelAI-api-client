// NovelAIAPI - Swift client library for the NovelAI image generation API
//
// Re-exports all public types and functions
//
// Client types:
//   - NovelAIClient: Main API client for image generation, augmentation, upscaling, etc.
//   - Logger: Protocol for logging warnings and errors during API operations.
//   - DefaultLogger: Default logger that prints to stderr.
//
// Schema types:
//   - GenerateParams, GenerateResult
//   - EncodeVibeParams, VibeEncodeResult
//   - AugmentParams, AugmentResult
//   - UpscaleParams, UpscaleResult
//   - AnlasBalance
//   - CharacterConfig, CharacterReferenceConfig
//   - ImageInput, VibeItem
//   - GenerateAction, CharRefMode, AugmentReqType
//
// Constants:
//   - Model, Sampler, NoiseSchedule
//   - DEFAULT_MODEL, DEFAULT_WIDTH, DEFAULT_HEIGHT, etc.
//
// Cost calculation:
//   - calculateGenerationCost, calculateAugmentCost, calculateUpscaleCost
//
// Error types:
//   - NovelAIError

@_exported import struct Foundation.Data
@_exported import struct Foundation.Date
