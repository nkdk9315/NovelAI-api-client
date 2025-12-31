"""
NovelAI API 統合テスト（手動実行専用）

⚠️ 重要:
- このテストは実際にNovelAI APIを呼び出します
- Anlasを消費します
- 自動実行されません（pytest通常実行では全スキップ）

実行方法:
    # 全テスト実行（確認プロンプトあり）
    uv run pytest novelai/test_api_integration.py -v --run-api

    # 特定のテストのみ
    uv run pytest novelai/test_api_integration.py::TestAPIIntegration::test_get_anlas_balance -v --run-api

    # 画像生成テスト（5 Anlas消費）
    uv run pytest novelai/test_api_integration.py::TestAPIIntegration::test_generate_basic -v --run-api

環境変数:
    NOVELAI_API_KEY: NovelAI APIキー（必須）
"""

import os
import pytest
import tempfile
from pathlib import Path
from datetime import datetime

# カスタムマーカー: --run-api フラグがないとスキップ
def pytest_configure(config):
    config.addinivalue_line(
        "markers", "api: mark test as API integration test (requires --run-api)"
    )


def pytest_addoption(parser):
    parser.addoption(
        "--run-api",
        action="store_true",
        default=False,
        help="Run API integration tests (consumes Anlas)"
    )


def pytest_collection_modifyitems(config, items):
    if config.getoption("--run-api"):
        # --run-api が指定された場合はスキップしない
        return
    skip_api = pytest.mark.skip(reason="API tests require --run-api option")
    for item in items:
        if "api" in item.keywords:
            item.add_marker(skip_api)


# =============================================================================
# フィクスチャ
# =============================================================================

@pytest.fixture(scope="module")
def client():
    """NovelAIClient インスタンス"""
    from .client import NovelAIClient
    
    api_key = os.environ.get("NOVELAI_API_KEY")
    if not api_key:
        pytest.skip("NOVELAI_API_KEY 環境変数が設定されていません")
    
    return NovelAIClient(api_key=api_key)


@pytest.fixture(scope="module")
def temp_output_dir():
    """一時出力ディレクトリ"""
    with tempfile.TemporaryDirectory() as tmpdir:
        yield Path(tmpdir)


# =============================================================================
# API統合テスト
# =============================================================================

@pytest.mark.api
class TestAPIIntegration:
    """
    NovelAI API統合テスト
    
    すべてのテストは @pytest.mark.api でマークされており、
    --run-api オプションなしでは自動的にスキップされます。
    """
    
    # -------------------------------------------------------------------------
    # アンラス残高取得（無料）
    # -------------------------------------------------------------------------
    
    def test_get_anlas_balance(self, client):
        """
        アンラス残高取得
        消費: 0 Anlas
        """
        result = client.get_anlas_balance()
        
        assert "fixed" in result
        assert "purchased" in result
        assert "total" in result
        assert "tier" in result
        
        assert isinstance(result["fixed"], int)
        assert isinstance(result["purchased"], int)
        assert isinstance(result["total"], int)
        assert result["tier"] in [0, 1, 2, 3]
        
        print(f"\n✅ アンラス残高: {result['total']} (fixed={result['fixed']}, purchased={result['purchased']})")
        print(f"   プラン: {['なし', 'Tablet', 'Scroll', 'Opus'][result['tier']]}")
    
    # -------------------------------------------------------------------------
    # バリデーションエラー（APIは呼ばれない）
    # -------------------------------------------------------------------------
    
    def test_validation_error_empty_prompt(self, client):
        """
        バリデーションエラー: 空のプロンプト
        消費: 0 Anlas（APIは呼ばれない）
        """
        from pydantic import ValidationError
        
        with pytest.raises(ValidationError) as exc_info:
            client.generate(prompt="")
        
        assert "prompt" in str(exc_info.value)
        print("\n✅ 空プロンプトのバリデーションエラーを確認")
    
    def test_validation_error_invalid_model(self, client):
        """
        バリデーションエラー: 無効なモデル
        消費: 0 Anlas（APIは呼ばれない）
        """
        from pydantic import ValidationError
        
        with pytest.raises(ValidationError) as exc_info:
            client.generate(prompt="test", model="invalid-model-name")
        
        assert "無効なモデル" in str(exc_info.value)
        print("\n✅ 無効モデルのバリデーションエラーを確認")
    
    def test_validation_error_pixels_exceed(self, client):
        """
        バリデーションエラー: ピクセル数超過
        消費: 0 Anlas（APIは呼ばれない）
        """
        from pydantic import ValidationError
        
        with pytest.raises(ValidationError) as exc_info:
            client.generate(prompt="test", width=1280, height=1280)
        
        assert "ピクセル数" in str(exc_info.value)
        print("\n✅ ピクセル超過のバリデーションエラーを確認")
    
    def test_validation_error_vibes_with_charref(self, client):
        """
        バリデーションエラー: vibesとcharacter_referenceの同時使用
        消費: 0 Anlas（APIは呼ばれない）
        """
        from pydantic import ValidationError
        from .dataclasses import CharacterReferenceConfig
        
        char_ref = CharacterReferenceConfig(image=b"dummy_image_data")
        
        with pytest.raises(ValidationError) as exc_info:
            client.generate(
                prompt="test",
                vibes=["some_vibe"],
                character_reference=char_ref
            )
        
        error_msg = str(exc_info.value)
        assert "vibes" in error_msg or "character_reference" in error_msg
        print("\n✅ vibes/charref同時使用のバリデーションエラーを確認")


@pytest.mark.api
class TestAPIGeneration:
    """
    画像生成テスト（Anlas消費あり）
    
    ⚠️ これらのテストはAnlasを消費します
    """
    
    # -------------------------------------------------------------------------
    # 基本画像生成（約5 Anlas）
    # -------------------------------------------------------------------------
    
    def test_generate_basic(self, client, temp_output_dir):
        """
        基本的な画像生成
        消費: 約5 Anlas（Opusプランなら無料）
        """
        # 生成前のアンラス
        before = client.get_anlas_balance()["total"]
        print(f"\n📊 生成前アンラス: {before}")
        
        result = client.generate(
            prompt="1girl, solo, upper body, simple background",
            negative_prompt="lowres, bad quality",
            width=512,  # 小さいサイズで消費を抑える
            height=768,
            steps=20,
            save_dir=temp_output_dir
        )
        
        # 結果検証
        assert result.image_data is not None
        assert len(result.image_data) > 0
        assert result.seed >= 0
        assert result.saved_path is not None
        assert result.saved_path.exists()
        
        # アンラス確認
        print(f"   生成後アンラス: {result.anlas_remaining}")
        print(f"   消費アンラス: {result.anlas_consumed}")
        print(f"   シード: {result.seed}")
        print(f"   保存先: {result.saved_path}")
        print("✅ 基本画像生成成功")
    
    def test_generate_with_seed(self, client, temp_output_dir):
        """
        シード固定での画像生成
        消費: 約5 Anlas（Opusプランなら無料）
        """
        fixed_seed = 42
        
        result = client.generate(
            prompt="1boy, solo, looking at viewer",
            width=512,
            height=512,
            steps=15,
            seed=fixed_seed,
            save_dir=temp_output_dir
        )
        
        assert result.seed == fixed_seed
        print(f"\n✅ シード固定生成成功 (seed={result.seed})")
    
    def test_generate_different_sampler(self, client, temp_output_dir):
        """
        異なるサンプラーでの生成
        消費: 約5 Anlas（Opusプランなら無料）
        """
        result = client.generate(
            prompt="landscape, mountains, sunset",
            width=768,
            height=512,
            steps=20,
            sampler="k_euler",
            noise_schedule="exponential",
            save_dir=temp_output_dir
        )
        
        assert result.image_data is not None
        print(f"\n✅ k_euler + exponential での生成成功")


@pytest.mark.api
class TestAPIVibeEncode:
    """
    Vibeエンコードテスト（2 Anlas消費）
    """
    
    def test_encode_vibe_basic(self, client, temp_output_dir):
        """
        基本的なVibeエンコード
        消費: 2 Anlas
        
        ⚠️ テスト用の画像ファイルが必要です
        """
        # テスト用画像を探す
        test_images = list(Path("/home/mur/workspace/novelAi/api").glob("*.png"))
        if not test_images:
            test_images = list(Path("/home/mur/workspace/novelAi/api").glob("*.webp"))
        
        if not test_images:
            pytest.skip("テスト用画像ファイルが見つかりません")
        
        test_image = test_images[0]
        print(f"\n📷 テスト画像: {test_image}")
        
        result = client.encode_vibe(
            image=test_image,
            information_extracted=0.7,
            strength=0.7,
            save_dir=temp_output_dir
        )
        
        assert result.encoding is not None
        assert len(result.encoding) > 0
        assert result.source_image_hash is not None
        assert len(result.source_image_hash) == 64
        assert result.created_at is not None
        
        print(f"   エンコード長: {len(result.encoding)} bytes")
        print(f"   画像ハッシュ: {result.source_image_hash[:16]}...")
        print(f"   消費アンラス: {result.anlas_consumed}")
        print("✅ Vibeエンコード成功")


# =============================================================================
# conftest.py として使用するためのフック
# =============================================================================

# このファイル内でpytestフックを定義しているため、
# conftest.pyにコピーするか、このファイルをconftest.pyとして使用してください。
# 
# もしくは、以下を novelai/conftest.py に追加:
#
# def pytest_addoption(parser):
#     parser.addoption(
#         "--run-api",
#         action="store_true",
#         default=False,
#         help="Run API integration tests (consumes Anlas)"
#     )
#
# def pytest_collection_modifyitems(config, items):
#     if config.getoption("--run-api"):
#         return
#     skip_api = pytest.mark.skip(reason="API tests require --run-api option")
#     for item in items:
#         if "api" in item.keywords:
#             item.add_marker(skip_api)
