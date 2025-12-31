"""
NovelAI テスト設定

pytest設定とカスタムフック
"""

import pytest


def pytest_configure(config):
    """カスタムマーカーを登録"""
    config.addinivalue_line(
        "markers", "api: mark test as API integration test (requires --run-api)"
    )


def pytest_addoption(parser):
    """--run-api オプションを追加"""
    parser.addoption(
        "--run-api",
        action="store_true",
        default=False,
        help="Run API integration tests (consumes Anlas)"
    )


def pytest_collection_modifyitems(config, items):
    """--run-api がない場合、api マーカー付きテストをスキップ"""
    if config.getoption("--run-api"):
        # --run-api が指定された場合はスキップしない
        return
    
    skip_api = pytest.mark.skip(
        reason="API tests require --run-api option (consumes Anlas)"
    )
    for item in items:
        if "api" in item.keywords:
            item.add_marker(skip_api)
