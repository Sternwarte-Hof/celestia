(comment) @comment.line

[
  "Configuration"
  "VirtualTexture"
  "Add"
  "Replace"
  "Modify"
  "Body"
  "ReferencePoint"
  "SurfaceObject"
  "AltSurface"
  "Location"
  "Star"
  "Barycenter"
  "Galaxy"
  "Globular"
  "Nebula"
  "OpenCluster"
  "galaxy"
  "globular"
  "nebula"
  "opencluster"
] @keyword.control

(property_entry key: (identifier) @property)
(property_entry key: (string) @property)
(unit) @keyword.other.unit

(string) @string.quoted.double
(number) @constant.numeric
(boolean) @constant.language.boolean
