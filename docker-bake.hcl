variable "IMAGE" {
  description = "Image repository for the built images."
  default     = "yeetypete/bootc-jetson"
}

variable "VERSION" {
  description = "Version tag for the built images."
  default     = "v0.0.0"
}

variable "REVISION" {
  description = "Git commit SHA for image labels."
  default     = ""
}

variable "PUSH" {
  description = "Also push images to the registry (in addition to the local OCI archive)."
  default     = false
}

group "default" {
  targets = ["jetson-orin", "jetson-thor"]
}

target "_common" {
  pull = true
  labels = {
    "org.opencontainers.image.version"  = trimprefix(VERSION, "v")
    "org.opencontainers.image.revision" = REVISION
  }
  # On release we emit the registry push and the local OCI archive from the same
  # build so they share a manifest digest.
  output = PUSH ? [
    "type=docker,compression=zstd",
    "type=oci,dest=image.oci,compression=zstd",
    "type=registry,compression=zstd",
    ] : [
    "type=docker,compression=zstd",
    "type=oci,dest=image.oci,compression=zstd",
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
  inherits   = ["_common"]
  context    = "./orin"
  dockerfile = "Dockerfile"
  platforms  = ["linux/arm64"]
  contexts = {
    jetson-tools = "./jetson-tools"
  }
  tags = [
    "${IMAGE}:orin-jp7.2",
    "${IMAGE}:orin-jp7.2-${trimprefix(VERSION, "v")}",
  ]
}

target "jetson-thor" {
  inherits   = ["_common"]
  context    = "./thor"
  dockerfile = "Dockerfile"
  platforms  = ["linux/arm64"]
  contexts = {
    jetson-tools = "./jetson-tools"
  }
  tags = [
    "${IMAGE}:thor-jp7.2",
    "${IMAGE}:thor-jp7.2-${trimprefix(VERSION, "v")}",
  ]
}
