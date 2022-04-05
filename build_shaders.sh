SHADERS_DEFERRED_G="eruptrace_deferred/src/geometry_pass/shaders"
SHADERS_PURE="eruptrace_pure/src/shaders"
SHADERS_RENDER_SURFACE="src/shaders"

glslc $SHADERS_DEFERRED_G/mesh.vert -o $SHADERS_DEFERRED_G/mesh_vert.spv
glslc $SHADERS_DEFERRED_G/mesh.frag -o $SHADERS_DEFERRED_G/mesh_frag.spv
glslc $SHADERS_PURE/image.vert -o $SHADERS_PURE/image_vert.spv
glslc $SHADERS_PURE/image.frag -o $SHADERS_PURE/image_frag.spv
glslc $SHADERS_RENDER_SURFACE/surface.vert -o $SHADERS_RENDER_SURFACE/surface_vert.spv
glslc $SHADERS_RENDER_SURFACE/surface.frag -o $SHADERS_RENDER_SURFACE/surface_frag.spv
