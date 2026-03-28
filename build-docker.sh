#!/bin/bash
# 构建 Docker 镜像的便捷脚本

set -e

IMAGE_NAME="rlist"
IMAGE_TAG="latest"

echo "Building Docker image: ${IMAGE_NAME}:${IMAGE_TAG}..."

docker build -t ${IMAGE_NAME}:${IMAGE_TAG} .

echo ""
echo "======================================"
echo "Docker image built successfully!"
echo "======================================"
echo ""
echo "Image: ${IMAGE_NAME}:${IMAGE_TAG}"
echo ""
echo "运行方式:"
echo "  docker run -d --name rlist -p 10000:10000 rlist"
echo ""
echo "或者挂载存储目录:"
echo "  docker run -d --name rlist -p 10000:10000 -v /path/to/storage:/app/data rlist"
echo ""
echo "查看日志:"
echo "  docker logs -f rlist"
echo ""
echo "停止容器:"
echo "  docker stop rlist && docker rm rlist"
echo ""
