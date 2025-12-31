## 2025-02-18 - Zip Bomb Protection in API Client
**Vulnerability:** The API client was vulnerable to Decompression Bombs (Zip Bombs). It was reading files from zip archives provided by the remote server directly into memory without checking their uncompressed size.
**Learning:** Even when consuming APIs from trusted providers, client libraries should practice defensive coding. A compromised upstream service or a Man-in-the-Middle attack could deliver a malicious payload causing Denial of Service (OOM) on the client side.
**Prevention:** Always check `ZipInfo.file_size` against a reasonable limit before calling `read()` or extracting files from a zip archive.
