#!/bin/bash -e

# Application name
APP_NAME="rustyip"

# Docker Hub username
DOCKER_HUB="richardsondev"

# Azure Container Registry name (e.g., youracrname, not the full FQDN)
ACR_NAME="richardsondev"

# Platforms to build
PLATFORMS=("linux/amd64" "linux/arm64" "linux/arm/v7")
# Corresponding short architecture tags for images
ARCH_TAGS=("amd64" "arm64" "armv7")

# Ensure buildx builder is ready
echo "Setting up Docker buildx builder..."
docker buildx create --use --name mybuilder || true
docker buildx use mybuilder
docker buildx inspect --bootstrap

# Step 1: Build and push multi-arch image to Docker Hub
echo ""
echo "--------------------------------------------------"
echo "Step 1: Building and pushing to Docker Hub"
echo "--------------------------------------------------"
DH_IMAGE_LATEST="$DOCKER_HUB/$APP_NAME:latest"
echo "Building and pushing $DH_IMAGE_LATEST (platforms: ${PLATFORMS[*]})..."
docker buildx build --platform $(IFS=,; echo "${PLATFORMS[*]}") -t $DH_IMAGE_LATEST --push .
echo "Docker Hub push complete for $DH_IMAGE_LATEST."

# Step 2: Azure Container Registry Processing
echo ""
echo "--------------------------------------------------"
echo "Step 2: Processing images for Azure Container Registry ($ACR_NAME)"
echo "--------------------------------------------------"

# Login to ACR
# The original script used /root/bin/az. This is a more generic attempt.
# Ensure you are logged in to ACR if this fails or 'az' is not available.
if command -v az &> /dev/null; then
    echo "Attempting Azure CLI login to ACR: $ACR_NAME.azurecr.io"
    az acr login --name $ACR_NAME
else
    echo "Warning: Azure CLI 'az' not found. Please ensure you are logged in to ACR ($ACR_NAME.azurecr.io) manually."
    # You might want to add a pause here if running interactively:
    # read -p "Press Enter to continue after manual login, or Ctrl+C to abort..."
fi

ACR_FQDN="$ACR_NAME.azurecr.io"
ACR_IMAGE_BASE="$ACR_FQDN/$APP_NAME"
MANIFEST_IMAGES_FOR_ACR=()

echo ""
echo "Building and pushing architecture-specific images to ACR..."
for i in "${!PLATFORMS[@]}"; do
    platform=${PLATFORMS[$i]}
    arch_tag=${ARCH_TAGS[$i]}
    local_build_tag="$APP_NAME:$arch_tag-localbuild" # Unique tag for local buildx output
    acr_arch_image="$ACR_IMAGE_BASE:$arch_tag"

    echo ""
    echo "Building $APP_NAME for $platform (output to local daemon as $local_build_tag)..."
    docker buildx build --platform $platform --load -t $local_build_tag .
    
    echo "Tagging $local_build_tag as $acr_arch_image"
    docker tag $local_build_tag $acr_arch_image
    
    echo "Pushing $acr_arch_image to ACR..."
    docker push $acr_arch_image
    
    MANIFEST_IMAGES_FOR_ACR+=("$acr_arch_image")
    echo "Successfully pushed $acr_arch_image."
done

# Step 3: Create and push the multi-arch manifest to ACR with 'multi' tag
echo ""
echo "--------------------------------------------------"
echo "Step 3: Creating and pushing multi-arch manifest to ACR"
echo "--------------------------------------------------"
MULTI_ARCH_MANIFEST_ACR="$ACR_IMAGE_BASE:multi"
echo "Creating manifest $MULTI_ARCH_MANIFEST_ACR for images: ${MANIFEST_IMAGES_FOR_ACR[*]}"
docker manifest create $MULTI_ARCH_MANIFEST_ACR "${MANIFEST_IMAGES_FOR_ACR[@]}"

echo "Pushing manifest $MULTI_ARCH_MANIFEST_ACR to ACR..."
docker manifest push --purge $MULTI_ARCH_MANIFEST_ACR # --purge removes local manifest list after push
echo "Successfully pushed multi-arch manifest $MULTI_ARCH_MANIFEST_ACR."

# Step 4: Tag the amd64 image as 'latest' for ACR and push
echo ""
echo "--------------------------------------------------"
echo "Step 4: Tagging and pushing amd64 image as 'latest' to ACR"
echo "--------------------------------------------------"
AMD64_ACR_IMAGE="$ACR_IMAGE_BASE:amd64" # This was pushed in the loop
LATEST_ACR_TAG="$ACR_IMAGE_BASE:latest"

echo "Tagging $AMD64_ACR_IMAGE as $LATEST_ACR_TAG"
docker tag $AMD64_ACR_IMAGE $LATEST_ACR_TAG

echo "Pushing $LATEST_ACR_TAG (which is the amd64 image) to ACR..."
docker push $LATEST_ACR_TAG
echo "Successfully pushed $LATEST_ACR_TAG."

echo ""
echo "--------------------------------------------------"
echo "Azure Container Registry processing complete."
echo "Summary of images pushed to $ACR_FQDN/$APP_NAME:"
echo "  Multi-arch manifest: $APP_NAME:multi (includes ${ARCH_TAGS[*]})"
echo "  'latest' tag (amd64): $APP_NAME:latest"
for arch_tag in "${ARCH_TAGS[@]}"; do
    echo "  Single architecture: $APP_NAME:$arch_tag"
done
echo "--------------------------------------------------"
