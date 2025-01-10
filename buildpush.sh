#!/bin/bash -e

# Application name
APP_NAME="rustyip"

# Docker Hub and Azure Container Registry (ACR) details
DOCKER_HUB="richardsondev"
ACR_NAME="richardsondev"

# Step 1: Setup Docker Buildx
# Create and use the buildx builder, if not already created
echo "Setting up Docker Buildx..."
docker buildx create --use --name mybuilder || true
docker buildx use mybuilder
docker buildx inspect --bootstrap

# Step 2: Build and Push Multi-Architecture Image to Docker Hub
echo "Building and pushing multi-architecture image to Docker Hub..."
docker buildx build \
  --platform linux/amd64,linux/arm64,linux/arm/v7 \
  --tag $DOCKER_HUB/$APP_NAME:latest \
  --push .

# Step 3: Tag and Push X64 Image to Azure Container Registry
echo "Tagging and pushing x64 image to Azure Container Registry..."
docker tag $DOCKER_HUB/$APP_NAME:latest $ACR_NAME.azurecr.io/$APP_NAME:latest

# Ensure Azure CLI is logged in
if ! command -v az &>/dev/null; then
  echo "Azure CLI is not installed. Please install it and try again."
  exit 1
fi

# Log in to ACR
echo "Logging into Azure Container Registry..."
az acr login --name $ACR_NAME

# Push the image to ACR
echo "Pushing image to Azure Container Registry..."
docker push $ACR_NAME.azurecr.io/$APP_NAME:latest

echo "Multi-architecture image build and push process complete!"
