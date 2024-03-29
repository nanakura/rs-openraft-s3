```
warning: `rs-s3-local` (bin "rs-s3-local") generated 1 warning
    Finished `dev` profile [unoptimized] target(s) in 0.21s
     Running `target\debug\rs-s3-local.exe`
[2024-03-29T07:16:34Z INFO  ntex_server::manager] Starting 12 workers
[2024-03-29T07:16:34Z INFO  ntex_server::net::builder] Starting "ntex-web-service-0.0.0.0:9000" service on 0.0.0.0:9000
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(0)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(1)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(8)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(2)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(3)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(4)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(6)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(7)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(5)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(9)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(10)
[2024-03-29T07:16:34Z INFO  ntex_server::wrk] Starting worker WorkerId(11)
[2024-03-29T07:16:34Z INFO  ntex_server::net::accept] Resuming socket listener on 0.0.0.0:9000 after back-pressure
[2024-03-29T07:18:10Z INFO  ntex::web::middleware::logger] 127.0.0.1:4588 "GET /?object-lock= HTTP/1.1" 200 102 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000617
[2024-03-29T07:18:10Z INFO  ntex::web::middleware::logger] 127.0.0.1:4588 "GET / HTTP/1.1" 200 102 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000436      
[2024-03-29T07:18:20Z INFO  ntex::web::middleware::logger] 127.0.0.1:4589 "PUT /new-bucket-076b0d47/ HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000552
[2024-03-29T07:18:20Z INFO  ntex::web::middleware::logger] 127.0.0.1:4589 "GET / HTTP/1.1" 200 228 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.001056
[2024-03-29T07:18:20Z INFO  ntex::web::middleware::logger] 127.0.0.1:4589 "GET /new-bucket-076b0d47/?object-lock= HTTP/1.1" 200 135 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000421
[2024-03-29T07:18:20Z INFO  ntex::web::middleware::logger] 127.0.0.1:4589 "GET /new-bucket-076b0d47/?delimiter=%2F&max-keys=1000&prefix= HTTP/1.1" 200 135 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000324
size:2781958
[2024-03-29T07:19:28Z INFO  ntex::web::middleware::logger] 127.0.0.1:4626 "PUT /new-bucket-076b0d47/ossroadmap.docx HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.425424
data\buckets\new-bucket-076b0d47\ossroadmap.docx.meta
[2024-03-29T07:19:28Z INFO  ntex::web::middleware::logger] 127.0.0.1:4626 "GET /new-bucket-076b0d47/?delimiter=%2F&max-keys=1000&prefix= HTTP/1.1" 200 261 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000765
[2024-03-29T07:24:01Z INFO  ntex::web::middleware::logger] 127.0.0.1:4739 "HEAD /new-bucket-076b0d47/ossroadmap.docx HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.001682
[2024-03-29T07:24:01Z INFO  ntex::web::middleware::logger] 127.0.0.1:4739 "GET /new-bucket-076b0d47/ossroadmap.docx HTTP/1.1" 200 2781958 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.017731
[2024-03-29T07:25:15Z INFO  ntex::web::middleware::logger] 127.0.0.1:4897 "GET /test/?location= HTTP/1.1" 404 0 "-" "MinIO (windows; amd64) minio-go/v7.0.69" 0.000251
[2024-03-29T07:25:15Z INFO  ntex::web::middleware::logger] 127.0.0.1:4897 "PUT /test/ HTTP/1.1" 200 0 "-" "MinIO (windows; amd64) minio-go/v7.0.69" 0.000421     
size:2785913
[2024-03-29T07:25:16Z INFO  ntex::web::middleware::logger] 127.0.0.1:4897 "PUT /test/test.docx HTTP/1.1" 200 0 "-" "MinIO (windows; amd64) minio-go/v7.0.69" 0.428358
[2024-03-29T07:25:16Z INFO  ntex::web::middleware::logger] 127.0.0.1:4897 "GET /test/test.docx HTTP/1.1" 200 2785913 "-" "MinIO (windows; amd64) minio-go/v7.0.69" 0.017551
[2024-03-29T07:32:03Z INFO  ntex::web::middleware::logger] 127.0.0.1:4975 "POST /new-bucket-076b0d47/xxx.pdf?uploads= HTTP/1.1" 200 186 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000769
size:8388608
[2024-03-29T07:32:08Z INFO  ntex::web::middleware::logger] 127.0.0.1:4975 "PUT /new-bucket-076b0d47/xxx.pdf?partNumber=1&uploadId=592b4388-8821-4a3a-a914-7dbd4b9f6861 HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 4.238721
size:8388608
[2024-03-29T07:32:08Z INFO  ntex::web::middleware::logger] 127.0.0.1:4976 "PUT /new-bucket-076b0d47/xxx.pdf?partNumber=2&uploadId=592b4388-8821-4a3a-a914-7dbd4b9f6861 HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 4.210024
size:8388608
[2024-03-29T07:32:12Z INFO  ntex::web::middleware::logger] 127.0.0.1:4975 "PUT /new-bucket-076b0d47/xxx.pdf?partNumber=3&uploadId=592b4388-8821-4a3a-a914-7dbd4b9f6861 HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 4.282089
size:668466
[2024-03-29T07:32:12Z INFO  ntex::web::middleware::logger] 127.0.0.1:4975 "PUT /new-bucket-076b0d47/xxx.pdf?partNumber=5&uploadId=592b4388-8821-4a3a-a914-7dbd4b9f6861 HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.063748
size:8388608
[2024-03-29T07:32:12Z INFO  ntex::web::middleware::logger] 127.0.0.1:4976 "PUT /new-bucket-076b0d47/xxx.pdf?partNumber=4&uploadId=592b4388-8821-4a3a-a914-7dbd4b9f6861 HTTP/1.1" 200 0 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 4.304938
[2024-03-29T07:32:12Z INFO  ntex::web::middleware::logger] 127.0.0.1:4975 "POST /new-bucket-076b0d47/xxx.pdf?uploadId=592b4388-8821-4a3a-a914-7dbd4b9f6861 HTTP/1.1" 200 186 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000354
data\buckets\new-bucket-076b0d47\ossroadmap.docx.meta
data\buckets\new-bucket-076b0d47\xxx.pdf.meta
[2024-03-29T07:32:13Z INFO  ntex::web::middleware::logger] 127.0.0.1:4976 "GET /new-bucket-076b0d47/?delimiter=%2F&max-keys=1000&prefix= HTTP/1.1" 200 379 "-" "S3 Browser/11.4.5 (https://s3browser.com)" 0.000682


```

1. 初始化
   创建路径`tempPath + "/" + uploadID + "/"`

2. 上传
   路径：`tempPath + "/" + uploadId + "/" + partNumber + ".temp"`

   如果已存在则计算etag并返回
   如果不存在则在路径`dataPath + bucketName + "/" + objectKey + "/" + partNumber.part`写入分片

3. 合并
   最终合并位置为：`dataPath + bucketName + "/" + objectKey`
   如果存在则获取