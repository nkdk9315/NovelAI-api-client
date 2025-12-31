"""
DNS診断スクリプト
NovelAI APIへの接続をテスト
"""

import socket
import httpx

def test_dns():
    """DNS解決をテスト"""
    print("=== DNS Resolution Test ===")
    hostname = "image.novelai.net"
    
    try:
        ip = socket.gethostbyname(hostname)
        print(f"✓ DNS resolved: {hostname} -> {ip}")
        return True
    except socket.gaierror as e:
        print(f"✗ DNS resolution failed: {e}")
        print("\n考えられる原因:")
        print("1. WSLのDNS設定に問題がある")
        print("2. インターネット接続が切れている")
        print("3. ファイアウォールがDNSクエリをブロックしている")
        return False

def test_http_connection():
    """HTTP接続をテスト"""
    print("\n=== HTTP Connection Test ===")
    
    try:
        with httpx.Client(timeout=10.0) as client:
            response = client.get("https://www.google.com")
            print(f"✓ HTTP connection works (status: {response.status_code})")
            return True
    except Exception as e:
        print(f"✗ HTTP connection failed: {e}")
        return False

def test_novelai_connection():
    """NovelAI APIへの接続をテスト"""
    print("\n=== NovelAI API Connection Test ===")
    
    try:
        with httpx.Client(timeout=10.0) as client:
            # 認証なしでアクセス（401エラーが返ればサーバーは応答している）
            response = client.get("https://image.novelai.net/")
            print(f"✓ NovelAI server is reachable (status: {response.status_code})")
            return True
    except httpx.ConnectError as e:
        print(f"✗ Connection error: {e}")
        return False
    except Exception as e:
        print(f"✗ Error: {e}")
        return False

def main():
    print("NovelAI API 接続診断\n")
    
    dns_ok = test_dns()
    http_ok = test_http_connection()
    novelai_ok = test_novelai_connection()
    
    print("\n" + "=" * 50)
    print("診断結果:")
    print(f"  DNS解決: {'✓' if dns_ok else '✗'}")
    print(f"  HTTP接続: {'✓' if http_ok else '✗'}")
    print(f"  NovelAI API: {'✓' if novelai_ok else '✗'}")
    
    if not dns_ok:
        print("\n推奨される対処法:")
        print("1. WSLのDNS設定を確認:")
        print("   sudo nano /etc/resolv.conf")
        print("   以下を追加:")
        print("   nameserver 8.8.8.8")
        print("   nameserver 8.8.4.4")
        print("\n2. WSLを再起動:")
        print("   wsl --shutdown")
        print("   (その後WSLを再度起動)")

if __name__ == "__main__":
    main()
