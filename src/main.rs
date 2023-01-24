use wgpu_bootstrap::{
    window::Window,
    frame::Frame,
    application::Application,
    context::Context,
    geometry::icosphere,
    camera::Camera,
    wgpu,
    cgmath,
    default::Vertex,
    computation::Computation,
    texture::create_texture_bind_group,
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ComputeData {
    delta_time: f32,
    number_vertices: f32,
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Velocity {
    pub velocity: [f32; 3]
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Spring {
    pub inital_index: f32, // index du points duquel on part
    pub linked_index: f32, // à qui il est lié
    pub rest_length: f32,
}



// --------   CONSTANTES   --------
// ==================================================

// we want to change the size of the cloth, the number of vertices and the start position
const CLOTH_SIZE: f32 = 35.0;
const CLOTH_VERTICES_PER_ROW: u32 = 25; // the cloth is a square, the minimum is 2
const CLOTH_CENTER_X: f32 = 0.0;
const CLOTH_CENTER_Y: f32 = 10.0;
const CLOTH_CENTER_Z: f32 = 0.0;
// Sphere
const SPHERE_RADIUS: f32 = 10.0;
const SPHERE_CENTER_X: f32 = 0.0;
const SPHERE_CENTER_Y: f32 = 0.0;
const SPHERE_CENTER_Z: f32 = 0.0;

const VERTEX_MASS: f32 = 0.3;
// Springs
const STRUCTURAL_STIFFNESS: f32 = 20.0;
const SHEAR_STIFFNESS: f32 = 20.0;
const BEND_STIFFNESS: f32 = 10.0;
const STRUCTURAL_DAMPING: f32 = 1.0;
const SHEAR_DAMPING: f32 = 1.0;
const BEND_DAMPING: f32 = 0.1;
// ==================================================

struct MyApp {
    // "bindgroup" décrivent un ensemble de ressources et comment elles peuvent être accessibles par un shader. Ces ressources peuvent inclure des textures, des buffers de données, des samplers, etc.
    camera_bind_group: wgpu::BindGroup, // La camera_bind_group est utilisée pour stocker les informations de la caméra, comme la matrice de vue et la matrice de projection, qui peuvent être utilisées pour afficher la scène à partir d'un point de vue spécifique.
    texture_bind_group: wgpu::BindGroup, // La texture_bind_group est utilisée pour stocker les informations de la texture qui seront utilisées pour remplir le tissu, comme les images, les samplers, etc.
    // sphere
    sphere_pipeline: wgpu::RenderPipeline,
    sphere_vertex_buffer: wgpu::Buffer,
    sphere_index_buffer: wgpu::Buffer,
    sphere_indices: Vec<u16>,
    // cloth
    cloth_pipeline: wgpu::RenderPipeline,
    cloth_vertex_buffer: wgpu::Buffer,
    cloth_index_buffer: wgpu::Buffer,
    cloth_indices: Vec<u16>,
    // compute
    compute_pipeline: wgpu::ComputePipeline, //étape 2 pipeline
    forces_compute_pipeline: wgpu::ComputePipeline, //étape 2 pipeline
    compute_vertices_bind_group: wgpu::BindGroup,
    compute_data_bind_group: wgpu::BindGroup,
    compute_velocities_bind_group: wgpu::BindGroup,
    compute_data_buffer: wgpu::Buffer, //étape 3 buffer
    compute_data: ComputeData,
    // spring
    springs_bind_group: wgpu::BindGroup,
}

impl MyApp {
    fn new(context: &Context) -> Self { 


// --------   CAMERA   --------
// ==================================================
        let camera = Camera {
            eye: (70.0, 50.0, 10.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: context.get_aspect_ratio(),
            fovy: 20.0,
            znear: 0.1,
            zfar: 100.0, //E100
        };

        let (_camera_buffer, camera_bind_group) = camera.create_camera_bind_group(context); // create_camera_bind_group est une fonction de la librarie de LRK

// ==================================================


// --------   SPHERE   --------
// ==================================================
        let sphere_pipeline = context.create_render_pipeline(
            "Render Pipeline Sphere",
            include_str!("blue.wgsl"),
            &[Vertex::desc()],
            &[&context.camera_bind_group_layout], // 1 seul binding de la camera car l'autre possiible binding à mettre ici c'est la texture de la sphere mais ici on ne lui donne pas de texture mais juste une couleur uni
            wgpu::PrimitiveTopology::LineList // du coup on utilise lineList et pas TriangleList
        );

        // fonction LRK qui crée plusieurs Vec3 qui crée la sphere
        let (mut sphere_vertices, sphere_indices) = icosphere(4); 

        // Multiplying the position of each vertex by the radius of the sphere to change the radius of the sphere
        for vertex in sphere_vertices.iter_mut() {
            let mut posn = cgmath::Vector3::from(vertex.position);
            posn *= SPHERE_RADIUS as f32;
            vertex.position = posn.into()
        }

        // we change the center of the sphere by adding the center of the sphere to the position of each vertex. ........ mais ici vu que toutes les constantes valent 0, ca sert à riien mais si on veut décaler le centre de la sphere on peut en changeant les constantes
        for vertex in sphere_vertices.iter_mut() {
            vertex.position[0] += SPHERE_CENTER_X;
            vertex.position[1] += SPHERE_CENTER_Y;
            vertex.position[2] += SPHERE_CENTER_Z;
        }

        // creation des buffers pour la positions de chaques sommets(vertices)
        let sphere_vertex_buffer = context.create_buffer(
            &sphere_vertices,
            wgpu::BufferUsages::VERTEX
        );

        // creation des buffers pour la positions de chaques qui permettent de lier les vertices
        let sphere_index_buffer = context.create_buffer( // étapes 3 - buffer -  on crée l'indeces de buffer ici
            &sphere_indices,
            wgpu::BufferUsages::INDEX
        );

// ==================================================


// --------   CLOTH   --------
// ==================================================
        let texture = context.create_texture( //Econtext.create_srgb_texture
            "Football",
            include_bytes!("ball_skin.png"),
        );

        let texture_bind_group = create_texture_bind_group(context, &texture);

        let cloth_pipeline = context.create_render_pipeline( // creation du pipeline pour lier le shader à cette variable
            "Pipeline Cloth",
            include_str!("cloth.wgsl"),
            &[Vertex::desc()],
            &[
                &context.texture_bind_group_layout,
                &context.camera_bind_group_layout,
                ],
            wgpu::PrimitiveTopology::TriangleList // comment interpreter les vertices en les convertissant en triangle, on fait un triangle car on a une texture
        );
        
        
        // create the cloth ...... comme pour la sphere avec les icosphere mais ici n'existe pas donc on doit créer les vertex nous meme
        let mut cloth_vertices = Vec::new();
        let mut cloth_indices: Vec<u16> = Vec::new();
        
        // create the vertices
        for i in 0..CLOTH_VERTICES_PER_ROW { //Esans parenthese du cloth size
            for j in 0..CLOTH_VERTICES_PER_ROW {
                cloth_vertices.push(Vertex { 
                    position: [
                        CLOTH_CENTER_X + i as f32 * (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32) - (CLOTH_SIZE / 2.0),
                        CLOTH_CENTER_Y,
                        CLOTH_CENTER_Z + j as f32 * (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32) - (CLOTH_SIZE / 2.0),
                    ],
                    normal: [0.0, 0.0, 0.0],
                    tangent: [0.0, 0.0, 0.0],
                    tex_coords: [ // au liieu d'utiliser des couleurs on utilises des points pour binder la texture qu'on va mettre
                        i as f32 * (1.0 / (CLOTH_VERTICES_PER_ROW - 1) as f32), // correspond aux .png mais en relatif
                        j as f32 * (1.0 / (CLOTH_VERTICES_PER_ROW - 1) as f32),
                    ],
                });
            }
        }

        // create the indices
        for i in 0..CLOTH_VERTICES_PER_ROW - 1 { //Epareil pour les 2 premiers mais différent dans la maniere créer le carré
            for j in 0..CLOTH_VERTICES_PER_ROW - 1 {
                // first triangle
                cloth_indices.push((i * CLOTH_VERTICES_PER_ROW + j) as u16); // étgape 3 buffer on met les indices de chaque points du vetemetns que l'on voit draw
                cloth_indices.push((i * CLOTH_VERTICES_PER_ROW + j + 1) as u16);
                cloth_indices.push(((i + 1) * CLOTH_VERTICES_PER_ROW + j) as u16);
                // second triangle
                cloth_indices.push((i * CLOTH_VERTICES_PER_ROW + j + 1) as u16);
                cloth_indices.push(((i + 1) * CLOTH_VERTICES_PER_ROW + j + 1) as u16);
                cloth_indices.push(((i + 1) * CLOTH_VERTICES_PER_ROW + j) as u16);
            }
        }

        // set the default speed of the cloth
        let mut cloth_velocities: Vec<Velocity> = Vec::new();

        // Creating a vector of velocities for each vertex in the cloth. chaque sommet recoit une vitesse nulle
        for _i in cloth_vertices.iter_mut() { // Epareil
            cloth_velocities.push(Velocity {
                velocity: [0.0, 0.0, 0.0],
            });
        }

        // create a buffer for the cloth
        let cloth_vertex_buffer = context.create_buffer(
            &cloth_vertices,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE
        );
        let cloth_index_buffer = context.create_buffer(
            &cloth_indices,
            wgpu::BufferUsages::INDEX
        );
        let cloth_velocities_buffer = context.create_buffer(
            &cloth_velocities,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX
        );

// ==================================================


// --------   COMPUTE ET FORCE   --------
// ==================================================

        // create the compute pipeline
        let compute_pipeline = context.create_compute_pipeline( //on assigne le shader à une variable dans le fichier rust
            "Compute Pipeline",
            include_str!("compute.wgsl"),
        );
        // create the force compute pipeline
        let forces_compute_pipeline = context.create_compute_pipeline(
            "Forces Compute Pipeline",
            include_str!("forces_compute.wgsl")
        );

        // dans cette variables on lie 2 choses : cloth_vertex_buffer (les points 3D de chaque poiints du tissue) et compute_pipeline : le shader qui nous permet de calculer des choses sur les vertex
        let compute_vertices_bind_group = context.create_bind_group( // toutes ses pipelines dans la doc aller voir https://sotrh.github.io/learn-wgpu/beginner/tutorial3-pipeline/#how-do-we-use-the-shaders
            "Compute Vertices Bind Group",
            &compute_pipeline.get_bind_group_layout(0),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cloth_vertex_buffer.as_entire_binding(),
                },
            ],
        );

        // dans cette variables on lie 2 choses : cloth_velocities_buffercloth_velocities_buffer et compute_pipeline : le shader qui nous permet de calculer des choses sur les vertex
        let compute_velocities_bind_group = context.create_bind_group(
            "Compute Velocities Bind Group",
            &compute_pipeline.get_bind_group_layout(1),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cloth_velocities_buffer.as_entire_binding(),
                },
            ],
        );

        // compute data -----------------------------------------------------
        let compute_data = ComputeData {
            delta_time: 0.01,
            number_vertices: (CLOTH_VERTICES_PER_ROW*CLOTH_VERTICES_PER_ROW) as f32,

            sphere_radius: SPHERE_RADIUS,
            sphere_center_x: SPHERE_CENTER_X,
            sphere_center_y: SPHERE_CENTER_Y,
            sphere_center_z: SPHERE_CENTER_Z,

            vertex_mass: VERTEX_MASS,

            structural_stiffness: STRUCTURAL_STIFFNESS,
            shear_stiffness: SHEAR_STIFFNESS,
            bend_stiffness: BEND_STIFFNESS,
            structural_damping: STRUCTURAL_DAMPING,
            shear_damping: SHEAR_DAMPING,
            bend_damping: BEND_DAMPING,
        };

        let compute_data_buffer = context.create_buffer( // étape 3 buffer
            &[compute_data],
            wgpu::BufferUsages::UNIFORM,
        );

        // dans cette variables on lie 2 choses : compute_data_buffer et compute_pipeline : le shader qui nous permet de calculer des choses sur les vertex
        let compute_data_bind_group = context.create_bind_group(
            "Compute Data Bind Group",
            &compute_pipeline.get_bind_group_layout(2), // il s'agit du pipelinelayout de la documentation
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: compute_data_buffer.as_entire_binding(),
                },
            ],
        );

// ==================================================


// --------   SPRINGS   --------
// ==================================================

        let mut springs: Vec<Spring> = Vec::new(); // variable dans laquelle on va mettre tout les springs ensembles

        for inital_index_iterate in 0..CLOTH_VERTICES_PER_ROW * CLOTH_VERTICES_PER_ROW {
            let col: i32 = (inital_index_iterate % CLOTH_VERTICES_PER_ROW) as i32; // s'incrémente de 1 à chaque nouvelle itération = nouvelle colonne et se réinitiliase en fin de ligne (principe de la colonne)
            let row: i32 = (inital_index_iterate / CLOTH_VERTICES_PER_ROW) as i32; // reste à 0 tant que on est pas passé au dessus du nombre de colonne max et la il rereste coincé à 1 et ainsi de suite (principe d'une ligne)
            // structural springs
            for j in [-1,1] as [i32; 2] { // boucle ou j vaut d'abord -1 puis +1
                // col +- 1
                let mut linked_index_iterate = row * CLOTH_VERTICES_PER_ROW as i32 + col + j;
                if col + j > CLOTH_VERTICES_PER_ROW as i32 - 1 || col + j < 0 {
                    linked_index_iterate = (CLOTH_VERTICES_PER_ROW * CLOTH_VERTICES_PER_ROW + 1) as i32;
                }
                springs.push(Spring {
                    inital_index: inital_index_iterate as f32,
                    linked_index: linked_index_iterate as f32,
                    rest_length: (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32),
                });
                // row +- 1
                linked_index_iterate = (row + j) * CLOTH_VERTICES_PER_ROW as i32 + col;
                if row + j > CLOTH_VERTICES_PER_ROW as i32 - 1 || row + j < 0 {
                    linked_index_iterate = (CLOTH_VERTICES_PER_ROW * CLOTH_VERTICES_PER_ROW + 1) as i32;
                }
                springs.push(Spring {
                    inital_index: inital_index_iterate as f32,
                    linked_index: linked_index_iterate as f32,
                    rest_length: (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32),
                });
            }

            
            // shear springs
            for j in [-1,1] as [i32; 2] {
                // col + j and row + j
                let mut linked_index_iterate = (row + j) * CLOTH_VERTICES_PER_ROW as i32 + col + j;
                if col + j > CLOTH_VERTICES_PER_ROW as i32 - 1 || col + j < 0 || row + j > CLOTH_VERTICES_PER_ROW as i32 - 1 || row + j < 0 {
                    linked_index_iterate = (CLOTH_VERTICES_PER_ROW * CLOTH_VERTICES_PER_ROW + 1) as i32;
                }
                springs.push(Spring {
                    inital_index: inital_index_iterate as f32,
                    linked_index: linked_index_iterate as f32,
                    rest_length: (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32) * 1.41421356237,
                });
                // col + j and row - j
                linked_index_iterate = (row - j) * CLOTH_VERTICES_PER_ROW as i32 + col + j;
                if col + j > CLOTH_VERTICES_PER_ROW as i32 - 1 || col + j < 0 || row - j > CLOTH_VERTICES_PER_ROW as i32 - 1 || row - j < 0 {
                    linked_index_iterate = (CLOTH_VERTICES_PER_ROW * CLOTH_VERTICES_PER_ROW + 1) as i32;
                }
                springs.push(Spring {
                    inital_index: inital_index_iterate as f32,
                    linked_index: linked_index_iterate as f32,
                    rest_length: (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32) * 1.41421356237,
                });
            }
            // bend springs
            for j in [-1,1] as [i32; 2] {
                // col +- 2j
                let mut linked_index_iterate = row * CLOTH_VERTICES_PER_ROW as i32 + col + 2 * j;
                if col + 2 * j > CLOTH_VERTICES_PER_ROW as i32 - 1 || col + 2 * j < 0 {
                    linked_index_iterate = (CLOTH_VERTICES_PER_ROW * CLOTH_VERTICES_PER_ROW + 1) as i32;
                }
                springs.push(Spring {
                    inital_index: inital_index_iterate as f32,
                    linked_index: linked_index_iterate as f32,
                    rest_length: (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32) * 2.0,
                });
                // row +- 2j
                linked_index_iterate = (row + 2 * j) * CLOTH_VERTICES_PER_ROW as i32 + col;
                if row + 2 * j > CLOTH_VERTICES_PER_ROW as i32 - 1 || row + 2 * j < 0 {
                    linked_index_iterate = (CLOTH_VERTICES_PER_ROW * CLOTH_VERTICES_PER_ROW + 1) as i32;
                }
                springs.push(Spring {
                    inital_index: inital_index_iterate as f32,
                    linked_index: linked_index_iterate as f32,
                    rest_length: (CLOTH_SIZE / (CLOTH_VERTICES_PER_ROW - 1) as f32) * 2.0,
                });
            }
            
        }

        // create a buffer for the springs
        let springs_buffer = context.create_buffer(
            springs.as_slice(),
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
        );
        // create a bind group for the springs
        let springs_bind_group = context.create_bind_group(
            "Sping Bind Group",
            &compute_pipeline.get_bind_group_layout(3),
            &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: springs_buffer.as_entire_binding(),
                },
            ]
        );
// ==================================================



        return Self { // on ajoute les renderpipelines, les bindgroup et les buffer à MyApp.... équiavalent à tout en haut
            camera_bind_group,
            texture_bind_group,
            // sphere
            sphere_pipeline,
            sphere_vertex_buffer,
            sphere_index_buffer,
            sphere_indices,
            // cloth
            cloth_pipeline,
            cloth_vertex_buffer,
            cloth_index_buffer,
            cloth_indices,
            // compute
            compute_pipeline,
            forces_compute_pipeline,
            compute_vertices_bind_group,
            compute_velocities_bind_group,
            compute_data_bind_group,
            compute_data_buffer,
            compute_data,
            // springs
            springs_bind_group,
        };
    }
    
}

impl Application for MyApp {

// --------   RENDER   --------
// ==================================================
    fn render(&self, context: &Context) -> Result<(), wgpu::SurfaceError> {
        let mut frame = Frame::new(context)?;
        
        {
            let mut render_pass = frame.begin_render_pass(wgpu::Color {r: 0.85, g: 0.85, b: 0.85, a: 1.0});
            // render the sphere
            render_pass.set_pipeline(&self.sphere_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.sphere_vertex_buffer.slice(..)); // set_vertex_buffer takes two parameters. The first is what buffer slot to use for this vertex buffer. You can have multiple vertex buffers set at a time.

            //The second parameter is the slice of the buffer to use. You can store as many objects in a buffer as your hardware allows, so slice allows us to specify which portion of the buffer to use. We use .. to specify the entire buffer.

            // le premier argument c'est le slot pris dans le buffer
            render_pass.set_index_buffer(self.sphere_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.sphere_indices.len() as u32, 0, 0..1); // dans la doc il utilise sphere_indices.len() en le mettant dans une variable

            // render the cloth as a triangle list
            render_pass.set_pipeline(&self.cloth_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.cloth_vertex_buffer.slice(..)); // slice(..) est un raccourci de "cloth_vertex_buffer.slice(0..cloth_vertex_buffer.len())"
            render_pass.set_index_buffer(self.cloth_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.cloth_indices.len() as u32, 0, 0..1);
        }
        frame.present();

        Ok(())
    }

// ==================================================


// --------   UPDATE   --------
// ==================================================
    fn update(&mut self, context: &Context, delta_time: f32) {
        // update the compute data
        let compute_data = ComputeData {
            delta_time,
            number_vertices: (CLOTH_VERTICES_PER_ROW*CLOTH_VERTICES_PER_ROW) as f32,
            sphere_radius: SPHERE_RADIUS,
            sphere_center_x: SPHERE_CENTER_X,
            sphere_center_y: SPHERE_CENTER_Y,
            sphere_center_z: SPHERE_CENTER_Z,
            vertex_mass: VERTEX_MASS,
            structural_stiffness: STRUCTURAL_STIFFNESS,
            shear_stiffness: SHEAR_STIFFNESS,
            bend_stiffness: BEND_STIFFNESS,
            structural_damping: STRUCTURAL_DAMPING,
            shear_damping: SHEAR_DAMPING,
            bend_damping: BEND_DAMPING,
        };
        context.update_buffer(&self.compute_data_buffer, &[compute_data]);

        let mut computation = Computation::new(context);

        {
            let mut compute_pass = computation.begin_compute_pass();
            // calculate the forces
            compute_pass.set_pipeline(&self.forces_compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_vertices_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.compute_velocities_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.compute_data_bind_group, &[]);
            compute_pass.set_bind_group(3, &self.springs_bind_group, &[]);
            compute_pass.dispatch_workgroups(((CLOTH_VERTICES_PER_ROW*CLOTH_VERTICES_PER_ROW) as f64/128.0).ceil() as u32, 1, 1);

            // update the positions and collisions
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(0, &self.compute_vertices_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.compute_velocities_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.compute_data_bind_group, &[]);
            compute_pass.set_bind_group(3, &self.springs_bind_group, &[]);
            compute_pass.dispatch_workgroups(((CLOTH_VERTICES_PER_ROW*CLOTH_VERTICES_PER_ROW) as f32/128.0).ceil() as u32, 1, 1);
        }
        computation.submit();
    }
// ==================================================

}

fn main() {
    let window = Window::new();


    let context = window.get_context();

    let my_app = MyApp::new(context);

    window.run(my_app);
}