variable "REPOSITORY" {
  description = "Repository the images are tagged in."
  default     = "yeetypete/bootc-jetson"
}

variable "VERSION" {
  description = "Version of the bootc images."
  default     = "v0.0.0"
}

variable "REVISION" {
  description = "Git commit SHA of the bootc images."
  default     = ""
}

variable "JETPACK" {
  description = "JetPack release the images target."
  default     = "7.2"
}

variable "RELEASE" {
  description = "Tag the images as a release."
  default     = false
}

variable "PUSH" {
  description = "Also push the images to the registry."
  default     = false
}

function "tags" {
  params = [variant]
  result = RELEASE ? [
    "${REPOSITORY}:${variant}-jp${JETPACK}",
    "${REPOSITORY}:${variant}-jp${JETPACK}-${trimprefix(VERSION, "v")}",
    "${REPOSITORY}:${variant}-jp${JETPACK}-${substr(REVISION, 0, 7)}",
    ] : [
    REVISION != "" ? "${REPOSITORY}:${variant}-jp${JETPACK}-${substr(REVISION, 0, 7)}" : "${REPOSITORY}:${variant}-jp${JETPACK}",
  ]
}

group "default" {
  targets = ["jetson-orin", "jetson-thor"]
}

target "_common" {
  pull      = true
  platforms = ["linux/arm64"]
  labels = {
    "org.opencontainers.image.source"   = "https://github.com/yeetypete/bootc-jetson"
    "org.opencontainers.image.version"  = trimprefix(VERSION, "v")
    "org.opencontainers.image.revision" = REVISION
    "org.opencontainers.image.licenses" = "MIT"
  }
  # The registry push and the local OCI archive come from the same build so they
  # share a manifest digest.
  output = PUSH ? [
    "type=docker,compression=zstd,force-compression=true,oci-mediatypes=true",
    "type=oci,dest=image.oci,compression=zstd,force-compression=true",
    "type=registry,compression=zstd,force-compression=true,oci-mediatypes=true",
    ] : [
    "type=docker,compression=zstd,force-compression=true,oci-mediatypes=true",
    "type=oci,dest=image.oci,compression=zstd,force-compression=true",
  ]
  attest = [
    {
      type = "provenance"
      mode = "max"
    },
    {
      type = "sbom"
    }
  ]
}

target "jetson-orin" {
  inherits = ["_common"]
  context  = "./orin"
  tags     = tags("orin")
  labels = {
    "org.opencontainers.image.title"       = "bootc-jetson Orin"
    "org.opencontainers.image.description" = "bootc image for NVIDIA Jetson Orin on JetPack ${JETPACK}."
  }
}

target "jetson-thor" {
  inherits = ["_common"]
  context  = "./thor"
  tags     = tags("thor")
  labels = {
    "org.opencontainers.image.title"       = "bootc-jetson Thor"
    "org.opencontainers.image.description" = "bootc image for NVIDIA Jetson Thor on JetPack ${JETPACK}."
  }
}
