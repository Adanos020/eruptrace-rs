SHADERS_DEFERRED="eruptrace_deferred/src/shaders"
SHADERS_PURE="eruptrace_pure/src/shaders"
SHADERS_RENDER_SURFACE="src/shaders"

glslc $SHADERS_DEFERRED/mesh.vert -o $SHADERS_DEFERRED/mesh_vert.spv
glslc $SHADERS_DEFERRED/mesh.frag -o $SHADERS_DEFERRED/mesh_frag.spv
glslc $SHADERS_DEFERRED/lighting.vert -o $SHADERS_DEFERRED/lighting_vert.spv
glslc $SHADERS_DEFERRED/lighting.frag -o $SHADERS_DEFERRED/lighting_frag.spv -I./glsl_include/

glslc $SHADERS_PURE/image.vert -o $SHADERS_PURE/image_vert.spv
glslc $SHADERS_PURE/image.frag -o $SHADERS_PURE/image_frag.spv -I./glsl_include/

glslc $SHADERS_RENDER_SURFACE/gui_mesh.vert -o $SHADERS_RENDER_SURFACE/gui_mesh_vert.spv
glslc $SHADERS_RENDER_SURFACE/gui_mesh.frag -o $SHADERS_RENDER_SURFACE/gui_mesh_frag.spv
