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

group "default" {
  targets = ["jetson-orin"]
}

target "_common" {
  labels = {
    "org.opencontainers.image.version"  = VERSION
    "org.opencontainers.image.revision" = REVISION
  }
  secret = ["id=ostree-auth,env=OSTREE_AUTH"]
  output = ["type=registry"]
}

target "jetson-orin" {
  inherits   = ["_common"]
  dockerfile = "Dockerfile"
  context    = "./orin"
  platforms  = ["linux/arm64"]
  tags = [
    "${IMAGE}:orin-jp7.2-${VERSION}",
    "${IMAGE}:orin-jp7.2",
  ]
}
