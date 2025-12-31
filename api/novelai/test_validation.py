"""
NovelAI Client バリデーションテスト

Pydanticモデルのバリデーションエラーを包括的にテストする
"""

import pytest
from datetime import datetime
from pathlib import Path
from pydantic import ValidationError

from .models import (
    CharacterConfigModel,
    CharacterReferenceConfigModel,
    VibeEncodeResultModel,
    GenerateResultModel,
    GenerateParamsModel,
    EncodeVibeParamsModel,
    APIKeyModel,
)
from .constants import (
    MAX_PROMPT_CHARS,
    MAX_PIXELS,
    MAX_STEPS,
    MAX_SCALE,
    MAX_SEED,
    VALID_MODELS,
    VALID_SAMPLERS,
    VALID_NOISE_SCHEDULES,
)


# =============================================================================
# CharacterConfigModel テスト
# =============================================================================

class TestCharacterConfigModel:
    """CharacterConfigModel のバリデーションテスト"""
    
    def test_valid_basic(self):
        """正常: 基本的なキャラクター設定"""
        config = CharacterConfigModel(prompt="1girl, blonde hair")
        assert config.prompt == "1girl, blonde hair"
        assert config.center_x == 0.5
        assert config.center_y == 0.5
        assert config.negative_prompt == ""
    
    def test_valid_with_position(self):
        """正常: 位置指定あり"""
        config = CharacterConfigModel(
            prompt="1boy, blue eyes",
            center_x=0.3,
            center_y=0.7
        )
        assert config.center_x == 0.3
        assert config.center_y == 0.7
    
    def test_invalid_empty_prompt(self):
        """エラー: 空のプロンプト"""
        with pytest.raises(ValidationError) as exc_info:
            CharacterConfigModel(prompt="")
        assert "prompt" in str(exc_info.value)
    
    def test_invalid_prompt_too_long(self):
        """エラー: プロンプトが長すぎる"""
        long_prompt = "a" * (MAX_PROMPT_CHARS + 1)
        with pytest.raises(ValidationError) as exc_info:
            CharacterConfigModel(prompt=long_prompt)
        assert "prompt" in str(exc_info.value)
    
    def test_invalid_center_x_below_range(self):
        """エラー: center_x が範囲外 (< 0)"""
        with pytest.raises(ValidationError) as exc_info:
            CharacterConfigModel(prompt="test", center_x=-0.1)
        assert "center_x" in str(exc_info.value)
    
    def test_invalid_center_x_above_range(self):
        """エラー: center_x が範囲外 (> 1)"""
        with pytest.raises(ValidationError) as exc_info:
            CharacterConfigModel(prompt="test", center_x=1.1)
        assert "center_x" in str(exc_info.value)
    
    def test_invalid_center_y_below_range(self):
        """エラー: center_y が範囲外 (< 0)"""
        with pytest.raises(ValidationError) as exc_info:
            CharacterConfigModel(prompt="test", center_y=-0.5)
        assert "center_y" in str(exc_info.value)
    
    def test_invalid_center_y_above_range(self):
        """エラー: center_y が範囲外 (> 1)"""
        with pytest.raises(ValidationError) as exc_info:
            CharacterConfigModel(prompt="test", center_y=2.0)
        assert "center_y" in str(exc_info.value)
    
    def test_invalid_negative_prompt_too_long(self):
        """エラー: ネガティブプロンプトが長すぎる"""
        long_negative = "b" * (MAX_PROMPT_CHARS + 1)
        with pytest.raises(ValidationError) as exc_info:
            CharacterConfigModel(prompt="test", negative_prompt=long_negative)
        assert "negative_prompt" in str(exc_info.value)


# =============================================================================
# CharacterReferenceConfigModel テスト
# =============================================================================

class TestCharacterReferenceConfigModel:
    """CharacterReferenceConfigModel のバリデーションテスト"""
    
    def test_valid_bytes_image(self):
        """正常: バイトデータの画像"""
        config = CharacterReferenceConfigModel(image=b"\x89PNG\r\n\x1a\n...")
        assert config.fidelity == 1.0
        assert config.include_style is True
    
    def test_valid_with_options(self):
        """正常: オプション指定"""
        config = CharacterReferenceConfigModel(
            image=b"image_data",
            fidelity=0.8,
            include_style=False
        )
        assert config.fidelity == 0.8
        assert config.include_style is False
    
    def test_invalid_fidelity_below_range(self):
        """エラー: fidelity が範囲外 (< 0)"""
        with pytest.raises(ValidationError) as exc_info:
            CharacterReferenceConfigModel(image=b"data", fidelity=-0.1)
        assert "fidelity" in str(exc_info.value)
    
    def test_invalid_fidelity_above_range(self):
        """エラー: fidelity が範囲外 (> 1)"""
        with pytest.raises(ValidationError) as exc_info:
            CharacterReferenceConfigModel(image=b"data", fidelity=1.5)
        assert "fidelity" in str(exc_info.value)


# =============================================================================
# VibeEncodeResultModel テスト
# =============================================================================

class TestVibeEncodeResultModel:
    """VibeEncodeResultModel のバリデーションテスト"""
    
    @pytest.fixture
    def valid_hash(self):
        return "a" * 64  # 有効なSHA256ハッシュ
    
    def test_valid_basic(self, valid_hash):
        """正常: 基本的なVibeエンコード結果"""
        result = VibeEncodeResultModel(
            encoding="base64data",
            model="nai-diffusion-4-5-full",
            information_extracted=0.7,
            strength=0.7,
            source_image_hash=valid_hash,
            created_at=datetime.now()
        )
        assert result.encoding == "base64data"
        assert result.model == "nai-diffusion-4-5-full"
    
    def test_invalid_empty_encoding(self, valid_hash):
        """エラー: 空のエンコーディング"""
        with pytest.raises(ValidationError) as exc_info:
            VibeEncodeResultModel(
                encoding="",
                model="nai-diffusion-4-5-full",
                information_extracted=0.7,
                strength=0.7,
                source_image_hash=valid_hash,
                created_at=datetime.now()
            )
        assert "encoding" in str(exc_info.value)
    
    def test_invalid_model(self, valid_hash):
        """エラー: 無効なモデル名"""
        with pytest.raises(ValidationError) as exc_info:
            VibeEncodeResultModel(
                encoding="data",
                model="invalid-model",
                information_extracted=0.7,
                strength=0.7,
                source_image_hash=valid_hash,
                created_at=datetime.now()
            )
        assert "無効なモデル" in str(exc_info.value)
    
    def test_invalid_information_extracted_range(self, valid_hash):
        """エラー: information_extracted が範囲外"""
        with pytest.raises(ValidationError) as exc_info:
            VibeEncodeResultModel(
                encoding="data",
                model="nai-diffusion-4-5-full",
                information_extracted=1.5,
                strength=0.7,
                source_image_hash=valid_hash,
                created_at=datetime.now()
            )
        assert "information_extracted" in str(exc_info.value)
    
    def test_invalid_hash_format(self):
        """エラー: 無効なハッシュ形式"""
        with pytest.raises(ValidationError) as exc_info:
            VibeEncodeResultModel(
                encoding="data",
                model="nai-diffusion-4-5-full",
                information_extracted=0.7,
                strength=0.7,
                source_image_hash="invalid-hash",
                created_at=datetime.now()
            )
        assert "source_image_hash" in str(exc_info.value)
    
    def test_invalid_hash_wrong_length(self):
        """エラー: ハッシュの長さが違う"""
        with pytest.raises(ValidationError) as exc_info:
            VibeEncodeResultModel(
                encoding="data",
                model="nai-diffusion-4-5-full",
                information_extracted=0.7,
                strength=0.7,
                source_image_hash="a" * 32,  # MD5の長さ
                created_at=datetime.now()
            )
        assert "source_image_hash" in str(exc_info.value)


# =============================================================================
# GenerateResultModel テスト
# =============================================================================

class TestGenerateResultModel:
    """GenerateResultModel のバリデーションテスト"""
    
    def test_valid_basic(self):
        """正常: 基本的な生成結果"""
        result = GenerateResultModel(
            image_data=b"\x89PNG\r\n\x1a\n...",
            seed=12345
        )
        assert result.seed == 12345
        assert result.anlas_remaining is None
    
    def test_valid_with_anlas(self):
        """正常: アンラス情報あり"""
        result = GenerateResultModel(
            image_data=b"png_data",
            seed=12345,
            anlas_remaining=1000,
            anlas_consumed=5
        )
        assert result.anlas_remaining == 1000
        assert result.anlas_consumed == 5
    
    def test_invalid_seed_negative(self):
        """エラー: シードが負"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateResultModel(image_data=b"data", seed=-1)
        assert "seed" in str(exc_info.value)
    
    def test_invalid_seed_too_large(self):
        """エラー: シードが最大値を超える"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateResultModel(image_data=b"data", seed=MAX_SEED + 1)
        assert "seed" in str(exc_info.value)
    
    def test_invalid_anlas_negative(self):
        """エラー: アンラスが負"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateResultModel(
                image_data=b"data",
                seed=123,
                anlas_remaining=-100
            )
        assert "anlas_remaining" in str(exc_info.value)


# =============================================================================
# GenerateParamsModel テスト
# =============================================================================

class TestGenerateParamsModel:
    """GenerateParamsModel のバリデーションテスト"""
    
    # --- 基本テスト ---
    
    def test_valid_minimal(self):
        """正常: 最小限のパラメータ"""
        params = GenerateParamsModel(prompt="1girl")
        assert params.prompt == "1girl"
        assert params.action == "generate"
        assert params.width == 832
        assert params.height == 1216
    
    def test_valid_full_params(self):
        """正常: すべてのパラメータ指定"""
        params = GenerateParamsModel(
            prompt="1girl, masterpiece",
            negative_prompt="lowres, bad quality",
            model="nai-diffusion-4-5-curated",
            width=1024,
            height=1024,
            steps=28,
            scale=6.0,
            seed=42,
            sampler="k_euler",
            noise_schedule="exponential"
        )
        assert params.model == "nai-diffusion-4-5-curated"
        assert params.steps == 28
    
    # --- プロンプトバリデーション ---
    
    def test_valid_empty_prompt(self):
        """正常: 空のプロンプト（キャラクター指定時用）"""
        params = GenerateParamsModel(prompt="")
        assert params.prompt == ""
    
    def test_invalid_prompt_too_long(self):
        """エラー: プロンプトが長すぎる"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="a" * (MAX_PROMPT_CHARS + 1))
        assert "prompt" in str(exc_info.value)
    
    def test_invalid_negative_prompt_too_long(self):
        """エラー: ネガティブプロンプトが長すぎる"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(
                prompt="test",
                negative_prompt="x" * (MAX_PROMPT_CHARS + 1)
            )
        assert "negative_prompt" in str(exc_info.value)
    
    # --- モデル・サンプラーバリデーション ---
    
    def test_invalid_model(self):
        """エラー: 無効なモデル"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", model="invalid-model")
        error_msg = str(exc_info.value)
        assert "無効なモデル" in error_msg
    
    def test_invalid_sampler(self):
        """エラー: 無効なサンプラー"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", sampler="invalid_sampler")
        assert "無効なサンプラー" in str(exc_info.value)
    
    def test_invalid_noise_schedule(self):
        """エラー: 無効なノイズスケジュール"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", noise_schedule="invalid")
        assert "無効なノイズスケジュール" in str(exc_info.value)
    
    # --- サイズ・ステップバリデーション ---
    
    def test_invalid_width_too_small(self):
        """エラー: 幅が小さすぎる"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", width=32)
        assert "width" in str(exc_info.value)
    
    def test_invalid_width_not_multiple_of_64(self):
        """エラー: 幅が64の倍数でない"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", width=100)
        assert "64の倍数" in str(exc_info.value)
    
    def test_invalid_height_not_multiple_of_64(self):
        """エラー: 高さが64の倍数でない"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", height=500)
        assert "64の倍数" in str(exc_info.value)
    
    def test_invalid_pixels_exceed_limit(self):
        """エラー: ピクセル数が上限を超える"""
        # 1280 * 1280 = 1,638,400 > 1,048,576
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", width=1280, height=1280)
        assert "ピクセル数" in str(exc_info.value)
    
    def test_invalid_steps_too_low(self):
        """エラー: ステップ数が少なすぎる"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", steps=0)
        assert "steps" in str(exc_info.value)
    
    def test_invalid_steps_too_high(self):
        """エラー: ステップ数が多すぎる"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", steps=MAX_STEPS + 1)
        assert "steps" in str(exc_info.value)
    
    def test_invalid_scale_too_high(self):
        """エラー: スケールが大きすぎる"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", scale=MAX_SCALE + 1)
        assert "scale" in str(exc_info.value)
    
    def test_invalid_seed_negative(self):
        """エラー: シードが負"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", seed=-1)
        assert "seed" in str(exc_info.value)
    
    def test_invalid_seed_too_large(self):
        """エラー: シードが最大値を超える"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", seed=MAX_SEED + 1)
        assert "seed" in str(exc_info.value)
    
    # --- img2img バリデーション ---
    
    def test_invalid_img2img_without_source(self):
        """エラー: img2imgでsource_imageなし"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", action="img2img")
        assert "source_image" in str(exc_info.value)
    
    def test_valid_img2img_with_source(self):
        """正常: img2imgでsource_imageあり"""
        params = GenerateParamsModel(
            prompt="test",
            action="img2img",
            source_image=b"image_bytes"
        )
        assert params.action == "img2img"
    
    def test_invalid_img2img_strength_out_of_range(self):
        """エラー: img2img_strength が範囲外"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(
                prompt="test",
                action="img2img",
                source_image=b"data",
                img2img_strength=1.5
            )
        assert "img2img_strength" in str(exc_info.value)
    
    # --- クロスフィールドバリデーション ---
    
    def test_invalid_vibes_with_character_reference(self):
        """エラー: vibes と character_reference の同時使用"""
        char_ref = CharacterReferenceConfigModel(image=b"image_data")
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(
                prompt="test",
                vibes=["encoded_vibe"],
                character_reference=char_ref
            )
        assert "vibes" in str(exc_info.value) or "character_reference" in str(exc_info.value)
    
    def test_invalid_vibe_strengths_without_vibes(self):
        """エラー: vibes なしで vibe_strengths 指定"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(
                prompt="test",
                vibe_strengths=[0.7, 0.8]
            )
        assert "vibe_strengths" in str(exc_info.value)
    
    def test_invalid_vibe_info_extracted_without_vibes(self):
        """エラー: vibes なしで vibe_info_extracted 指定"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(
                prompt="test",
                vibe_info_extracted=[0.7]
            )
        assert "vibe_info_extracted" in str(exc_info.value)
    
    def test_invalid_vibe_strengths_length_mismatch(self):
        """エラー: vibes と vibe_strengths の長さ不一致"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(
                prompt="test",
                vibes=["vibe1", "vibe2"],
                vibe_strengths=[0.7]  # 長さ1、vibesは長さ2
            )
        assert "vibe_strengths" in str(exc_info.value)
    
    def test_invalid_vibe_info_extracted_length_mismatch(self):
        """エラー: vibes と vibe_info_extracted の長さ不一致"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(
                prompt="test",
                vibes=["vibe1"],
                vibe_info_extracted=[0.7, 0.8]  # 長さ2、vibesは長さ1
            )
        assert "vibe_info_extracted" in str(exc_info.value)
    
    def test_valid_vibes_with_matching_lengths(self):
        """正常: vibes と関連パラメータの長さが一致"""
        params = GenerateParamsModel(
            prompt="test",
            vibes=["vibe1", "vibe2"],
            vibe_strengths=[0.7, 0.8],
            vibe_info_extracted=[0.6, 0.7]
        )
        assert len(params.vibes) == 2
        assert len(params.vibe_strengths) == 2


# =============================================================================
# EncodeVibeParamsModel テスト
# =============================================================================

class TestEncodeVibeParamsModel:
    """EncodeVibeParamsModel のバリデーションテスト"""
    
    def test_valid_basic(self):
        """正常: 基本的なVibeエンコードパラメータ"""
        params = EncodeVibeParamsModel(image=b"image_data")
        assert params.model == "nai-diffusion-4-5-full"
        assert params.information_extracted == 0.7
        assert params.strength == 0.7
    
    def test_valid_with_options(self):
        """正常: オプション指定"""
        params = EncodeVibeParamsModel(
            image=b"data",
            model="nai-diffusion-4-curated-preview",
            information_extracted=0.5,
            strength=0.9
        )
        assert params.model == "nai-diffusion-4-curated-preview"
    
    def test_invalid_model(self):
        """エラー: 無効なモデル"""
        with pytest.raises(ValidationError) as exc_info:
            EncodeVibeParamsModel(image=b"data", model="bad-model")
        assert "無効なモデル" in str(exc_info.value)
    
    def test_invalid_information_extracted_range(self):
        """エラー: information_extracted が範囲外"""
        with pytest.raises(ValidationError) as exc_info:
            EncodeVibeParamsModel(image=b"data", information_extracted=2.0)
        assert "information_extracted" in str(exc_info.value)
    
    def test_invalid_strength_negative(self):
        """エラー: strength が負"""
        with pytest.raises(ValidationError) as exc_info:
            EncodeVibeParamsModel(image=b"data", strength=-0.5)
        assert "strength" in str(exc_info.value)


# =============================================================================
# APIKeyModel テスト
# =============================================================================

class TestAPIKeyModel:
    """APIKeyModel のバリデーションテスト"""
    
    def test_valid_api_key(self):
        """正常: 有効なAPIキー"""
        key = APIKeyModel(api_key="pst-abcdef1234567890")
        assert key.api_key.startswith("pst-")
    
    def test_invalid_wrong_prefix(self):
        """エラー: プレフィックスが違う"""
        with pytest.raises(ValidationError) as exc_info:
            APIKeyModel(api_key="abc-1234567890")
        assert "api_key" in str(exc_info.value)
    
    def test_invalid_too_short(self):
        """エラー: APIキーが短すぎる"""
        with pytest.raises(ValidationError) as exc_info:
            APIKeyModel(api_key="pst-abc")
        assert "api_key" in str(exc_info.value)


# =============================================================================
# 境界値テスト
# =============================================================================

class TestBoundaryValues:
    """境界値のテスト"""
    
    def test_prompt_at_max_length(self):
        """境界: プロンプトが最大長ちょうど"""
        prompt = "a" * MAX_PROMPT_CHARS
        params = GenerateParamsModel(prompt=prompt)
        assert len(params.prompt) == MAX_PROMPT_CHARS
    
    def test_steps_at_min(self):
        """境界: ステップ数が最小値"""
        params = GenerateParamsModel(prompt="test", steps=1)
        assert params.steps == 1
    
    def test_steps_at_max(self):
        """境界: ステップ数が最大値"""
        params = GenerateParamsModel(prompt="test", steps=MAX_STEPS)
        assert params.steps == MAX_STEPS
    
    def test_scale_at_min(self):
        """境界: スケールが最小値"""
        params = GenerateParamsModel(prompt="test", scale=0.0)
        assert params.scale == 0.0
    
    def test_scale_at_max(self):
        """境界: スケールが最大値"""
        params = GenerateParamsModel(prompt="test", scale=MAX_SCALE)
        assert params.scale == MAX_SCALE
    
    def test_seed_at_min(self):
        """境界: シードが最小値"""
        params = GenerateParamsModel(prompt="test", seed=0)
        assert params.seed == 0
    
    def test_seed_at_max(self):
        """境界: シードが最大値"""
        params = GenerateParamsModel(prompt="test", seed=MAX_SEED)
        assert params.seed == MAX_SEED
    
    def test_pixels_at_max(self):
        """境界: ピクセル数が上限ちょうど"""
        # 1024 * 1024 = 1,048,576
        params = GenerateParamsModel(prompt="test", width=1024, height=1024)
        assert params.width * params.height == MAX_PIXELS
    
    def test_center_at_boundaries(self):
        """境界: 座標が0と1"""
        config = CharacterConfigModel(
            prompt="test",
            center_x=0.0,
            center_y=1.0
        )
        assert config.center_x == 0.0
        assert config.center_y == 1.0


# =============================================================================
# エラーメッセージテスト
# =============================================================================

class TestErrorMessages:
    """エラーメッセージの内容テスト"""
    
    def test_model_error_shows_valid_options(self):
        """モデルエラーに有効なオプションが表示される"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", model="bad-model")
        error_msg = str(exc_info.value)
        # 有効なモデルの一部が表示されることを確認
        assert "有効なモデル" in error_msg
    
    def test_sampler_error_shows_valid_options(self):
        """サンプラーエラーに有効なオプションが表示される"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", sampler="bad-sampler")
        error_msg = str(exc_info.value)
        assert "有効なサンプラー" in error_msg
    
    def test_dimension_error_shows_current_value(self):
        """寸法エラーに現在値が表示される"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", width=100)
        error_msg = str(exc_info.value)
        assert "100" in error_msg
    
    def test_pixel_error_shows_limit(self):
        """ピクセルエラーに上限が表示される"""
        with pytest.raises(ValidationError) as exc_info:
            GenerateParamsModel(prompt="test", width=1280, height=1280)
        error_msg = str(exc_info.value)
        assert "1,048,576" in error_msg or "1048576" in error_msg


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
