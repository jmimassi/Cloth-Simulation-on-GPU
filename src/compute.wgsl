struct Position {
    position_x: f32,
    position_y: f32,
    position_z: f32,
    normal_x: f32,
    normal_y: f32,
    normal_z: f32,
    tangent_x: f32,
    tangent_y: f32,
    tangent_z: f32,
    tex_coords_x: f32,
    tex_coords_y: f32,
}

struct Velocity {
    velocity_x: f32,
    velocity_y: f32,
    velocity_z: f32,
}

struct ComputeData {
    delta_time: f32,
    nb_vertices: f32,
    sphere_radius: f32,
    sphere_center_x: f32,
    sphere_center_y: f32,
    sphere_center_z: f32,
    vertex_mass: f32,
    structural_stiffness: f32,
    shear_stiffness: f32,
    bend_stiffness: f32,
    structural_damping: f32,
    shear_damping: f32,
    bend_damping: f32,
}

struct Spring {
    vertex_index_1: f32,
    vertex_index_2: f32,
    rest_length: f32,
}
// tout les bind group cad le lien entre les compute pipeline et les vertices, les velocities, les data, les springs
@group(0) @binding(0) var<storage, read_write> verticiesPositions: array<Position>; //positioons prédéfini grâce à toutes les boucles
@group(1) @binding(0) var<storage, read_write> verticiesVelocities: array<Velocity>; // vaut 0 au début pour tout les axes
@group(2) @binding(0) var<uniform> data: ComputeData; // toutes les valeurs de simulations
@group(3) @binding(0) var<storage, read> springsR: array<Spring>; // tout les indexes des springs

@compute @workgroup_size(128, 1, 1)
fn main(@builtin(global_invocation_id) param: vec3<u32>) {
    if (param.x >= u32(data.nb_vertices)) {
          return;
    }

    var spring = springsR[param.x];

    // toutes les particules avancent % de leur velocity
    verticiesPositions[param.x].position_x += verticiesVelocities[param.x].velocity_x * data.delta_time;
    verticiesPositions[param.x].position_y += verticiesVelocities[param.x].velocity_y * data.delta_time;
    verticiesPositions[param.x].position_z += verticiesVelocities[param.x].velocity_z * data.delta_time;


    let sphere_center = vec3<f32>(data.sphere_center_x, data.sphere_center_y, data.sphere_center_z);
    let sphere_radius = data.sphere_radius;

    // positions des sommets
    let position = vec3<f32>(verticiesPositions[param.x].position_x, verticiesPositions[param.x].position_y, verticiesPositions[param.x].position_z);

    // distance entre un point et le centre de la sphere, length c'est une formule magique un peu qui prend en param un vec3 de sphere_center et le vec3 des positions
    let distance = length(position - sphere_center);

    verticiesVelocities[param.x].velocity_x += 0.0;

    // si le points touche ou dépasse la sphère
    if (distance < sphere_radius) {
        // on trouve la normal entre le point et la sphere pour rebondir dans le sens iinverse ...... return a unit vector
        let normal = normalize(position - sphere_center);

        // on fait déplacer les points dans le sens de la normal et dans le sens inverse car sphere radius - distance
        verticiesPositions[param.x].position_x += normal.x * (sphere_radius - distance);
        verticiesPositions[param.x].position_y += normal.y * (sphere_radius - distance);
        verticiesPositions[param.x].position_z += normal.z * (sphere_radius - distance);

        verticiesVelocities[param.x].velocity_x = 0.0;
        verticiesVelocities[param.x].velocity_y = 0.0;
        verticiesVelocities[param.x].velocity_z = 0.0;
    }
}