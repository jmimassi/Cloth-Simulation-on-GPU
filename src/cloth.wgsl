// Vertex shader

struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> matrices: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) tangent: vec3<f32>,
    @location(3) tex_coords: vec2<f32>, // coordonnées des textures qu'on va bind sur les triangles
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>, //ca veut dire que c'est un vertex ............   the x and y of clip_position would be between 0-800 and 0-600 respectively with the y = 0 being the top of the screen. 
    @location(0) tex_coords: vec2<f32>, // The @location(0) bit tells WGPU to store the vec4 value returned by this function in the first color target. We'll get into what this is later.
    @location(1) normal: vec3<f32>,
}

@vertex // We are using @vertex to mark this function as a valid entry point for a vertex shader. We expect a u32 called in_vertex_index which gets its value from @builtin(vertex_index).
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = matrices.proj * matrices.view * vec4<f32>(model.position, 1.0);
    out.normal = model.normal;
    return out;
}

// Fragment shader
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment // c'est le fragment qui associe à chaque pixel du vertex une couleur
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords); // tout ses parametre sont en lien avec la camera
}