#!/bin/bash -e

# Application name
APP_NAME="rustyip"

# Step 1: Build the Docker image and push to Docker Hub
# This builds and pushes ARM32, ARM64, and X64 images
DOCKER_HUB="richardsondev"
docker buildx create --use --name mybuilder || true
docker buildx use mybuilder
docker buildx inspect --bootstrap
docker buildx build --platform linux/amd64,linux/arm64,linux/arm/v7 -t $DOCKER_HUB/$APP_NAME:latest --push .

# Step 2: Push the X64 image to Azure Container Registry
ACR_NAME="richardsondev"
ACR_REGISTRY="${ACR_REGISTRY:-richardsondev.azurecr.io}"
docker tag $APP_NAME $ACR_REGISTRY/$APP_NAME:latest
az acr login --name $ACR_NAME
docker push $ACR_REGISTRY/$APP_NAME:latest
