#![recursion_limit="512"]
//FlightGear is ran with this line of command argumments on the fgfs executable:
//fgfs.exe --aircraft=ufo --disable-panel --disable-sound --enable-hud --disable-random-objects --fdm=null --vc=0 --timeofday=noon --native-fdm=socket,in,30,,5500,udp
//fgfs.exe --aircraft=ufo --disable-panel --disable-sound --enable-hud --disable-random-objects --fdm=null --vc=0 --timeofday=noon --native-fdm=socket,in,60,,5500,udp


//Imports for flight control function
//Async std crossterm
use std::{
    io::{stdout, Write},
    time::Duration,
    time::Instant,
};
use futures::{future::FutureExt, select, StreamExt};
use futures_timer::Delay;
use crossterm::{
    event::{Event, EventStream, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
//Crossterm output printing
use crossterm::cursor;
use crossterm::terminal::{ Clear, ClearType};

//Specs
use specs::prelude::*;

//Coordinate conversions
extern crate coord_transforms;
use coord_transforms::prelude::*;

//Networking
use std::net::UdpSocket;

//Ellipsoid global variable
#[macro_use]
extern crate lazy_static;

//Exit program
use std::process;

//quaternion and vector matrix operations
extern crate nalgebra as na;
use na::{Matrix3, Vector3, UnitQuaternion, Quaternion}; 

//game loop
use std::thread;
use time::NumericalDuration;


mod common;
// //////Component Position
// #[derive(Debug)]
// struct Position
// {
//     ecef_vec: Vector3<f64>
// }
// impl Component for Position 
// {
//     type Storage = VecStorage<Self>;
// }

//////Component State Machine for keyboard
#[derive(Debug)]
struct KeyboardState
{
    thrust_up: bool,
    thrust_down: bool,

    left_rudder: bool,
    right_rudder: bool,
    zero_rudder: bool,

    roll_left: bool,
    roll_right: bool,
    zero_ailerons: bool,

    pitch_up: bool,
    pitch_down: bool,
    zero_elevators: bool,

    flaps_down: bool,
    zero_flaps: bool,

}
impl Component for KeyboardState
{
    type Storage = VecStorage<Self>;
}



//the point masses are elements making up the bodystructure 
#[derive(Debug)]
struct PointMass
{
    f_mass: f64,
    v_d_coords: Vector3<f64>, //"design position"
    v_local_inertia: Vector3<f64>,
    f_incidence: f64,
    f_dihedral: f64,
    f_area: f64,
    i_flap: i32,
    v_normal: Vector3<f64>,
    v_cg_coords: Vector3<f64> //"corrected position"

}

//////Component 
#[derive(Debug, Default)]
struct RigidBody
{
    mass: f64, //total mass
    m_inertia: Matrix3<f64>,
    m_inertia_inverse: Matrix3<f64>,
    v_position: Vector3<f64>,           // position in earth coordinates
    v_velocity: Vector3<f64>,           // velocity in earth coordinates
    v_velocity_body: Vector3<f64>,      // velocity in body coordinates
    v_angular_velocity: Vector3<f64>,   // angular velocity in body coordinates
    v_euler_angles: Vector3<f64>,   
    f_speed: f64,                       // speed (magnitude of the velocity)
    stalling: bool,
    flaps: bool,
    q_orientation: Quaternion<f64>,
    q_orientation_unit: UnitQuaternion<f64>,    // orientation in earth coordinates
    v_forces: Vector3<f64>,                     // total force on body
    thrustforce: f64,                           //magnitude of thrust
    v_moments: Vector3<f64>,                    // total moment (torque) on body

    element: Vec<PointMass>,                     //vector of point mass elements

    v_position_lla: Vector3<f64>,
    alt: f64,
    frame_count: f64 
}
impl Component for RigidBody
{
    type Storage = VecStorage<Self>;
}


//////Component FGNetFDM for networking
#[derive(Debug, Default)]
#[repr(C)] 
struct FGNetFDM
{
    version: u32, // increment when data values change
    padding: f32, // padding

    // // Positions
    longitude: f64, // geodetic (radians)
    latitude: f64, // geodetic (radians)
    altitude: f64, // above sea level (meters)
    agl: f32, // above ground level (meters)
    phi: f32, // roll (radians)
    theta: f32, // pitch (radians)
    psi: f32, // yaw or true heading (radians)
    alpha: f32, // angle of attack (radians)
    beta: f32, // side slip angle (radians)

    // // Velocities
    phidot: f32, // roll rate (radians/sec)
    thetadot: f32, // pitch rate (radians/sec)
    psidot: f32, // yaw rate (radians/sec)
    vcas: f32, // calibrated airspeed
    climb_rate: f32, // feet per second
    v_north: f32, // north velocity in local/body frame, fps
    v_east: f32, // east velocity in local/body frame, fps
    v_down: f32, // down/vertical velocity in local/body frame, fps
    v_body_u: f32, // ECEF velocity in body frame
    v_body_v: f32, // ECEF velocity in body frame 
    v_body_w: f32, // ECEF velocity in body frame
    
    // // Accelerations
    a_x_pilot: f32, // X accel in body frame ft/sec^2
    a_y_pilot: f32, // Y accel in body frame ft/sec^2
    a_z_pilot: f32, // Z accel in body frame ft/sec^2

    // // Stall
    stall_warning: f32, // 0.0 - 1.0 indicating the amount of stall
    slip_deg: f32, // slip ball deflection
    
    // // Engine status
    num_engines: u32, // Number of valid engines
    eng_state: [f32; 4], // Engine state (off, cranking, running)
    rpm: [f32; 4], // // Engine RPM rev/min
    fuel_flow: [f32; 4], // Fuel flow gallons/hr
    fuel_px: [f32; 4], // Fuel pressure psi
    egt: [f32; 4], // Exhuast gas temp deg F
    cht: [f32; 4], // Cylinder head temp deg F
    mp_osi: [f32; 4], // Manifold pressure
    tit: [f32; 4], // Turbine Inlet Temperature
    oil_temp: [f32; 4], // Oil temp deg F
    oil_px: [f32; 4], // Oil pressure psi

    // // Consumables
    num_tanks: u32, // Max number of fuel tanks
    fuel_quantity: [f32; 4], 

    // // Gear status
    num_wheels: u32, 
    wow: [f32; 3], 
    gear_pos: [f32; 3],
    gear_steer: [f32; 3],
    gear_compression: [f32; 3],

    // // Environment
    cur_time: f32, // current unix time
    warp: f32, // offset in seconds to unix time
    visibility: f32, // visibility in meters (for env. effects)

    // // Control surface positions (normalized values)
    elevator: f32,
    elevator_trim_tab: f32, 
    left_flap: f32,
    right_flap: f32,
    left_aileron: f32, 
    right_aileron: f32, 
    rudder: f32, 
    nose_wheel: f32,
    speedbrake: f32,
    spoilers: f32,
}
impl Component for FGNetFDM
{
    type Storage = VecStorage<Self>;
}
//for converting to slice of u8 
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8]
{
    ::std::slice::from_raw_parts((p as *const T) as *const u8,::std::mem::size_of::<T>(),)
}





//MASS PROPERTIES ONLY CALLED ONCE AT BEGGINING
fn calc_airplane_mass_properties(rigidbod: &mut RigidBody)
{
    //println!("{}", "calculating mass properties...");
    let mut inn: f64;
    let mut di: f64;

    //calculate the normal (perpendicular) vector to each lifting surface. This is needed for relative air velocity to find lift and drag.
    for  i in rigidbod.element.iter_mut()
    {
        inn = (i.f_incidence).to_radians();
        di = (i.f_dihedral).to_radians();
        i.v_normal = Vector3::new(inn.sin(), inn.cos() * di.sin(), inn.cos() * di.cos());
        i.v_normal = i.v_normal.normalize(); //not an issue here...
        //println!("{}",i.v_normal );
    }

    //calculate total mass
    let mut total_mass: f64 = 0.0;
    for i in rigidbod.element.iter()
    {
        total_mass = total_mass + i.f_mass;
    }
    //println!("Total mass: {}", total_mass);

    //calculate combined center of gravity location
    let mut first_moment_x: f64 = 0.0;
    let mut first_moment_y: f64 = 0.0;
    let mut first_moment_z: f64 = 0.0;
    for i in rigidbod.element.iter()
    {
        //X coord
        first_moment_x = first_moment_x + i.f_mass * i.v_d_coords.x;
        //y coord
        first_moment_y = first_moment_y + i.f_mass * i.v_d_coords.y;
        //z coord
        first_moment_z = first_moment_z + i.f_mass * i.v_d_coords.z;
        // vMoment = vMoment * i.f_mass * i.v_d_coords; //vector multiplcation not set up properly... oh well. even with nalgebra it panics
    }
    let v_moment = Vector3::new(first_moment_x, first_moment_y, first_moment_z); //remember there is a v_moments in rigid body.. we'll see how this plays out
    let cg = v_moment / total_mass; //operator overload works! Vector / scalar
    //println!("{}", v_moment);
    //println!("Combined center of gravity {:?}", cg);

    //calculate coordinates of each element with respect to the combined CG, relative position
    for i in rigidbod.element.iter_mut()
    {
        i.v_cg_coords.x = i.v_d_coords.x - cg.x;
        i.v_cg_coords.y = i.v_d_coords.y - cg.y;
        i.v_cg_coords.z = i.v_d_coords.z - cg.z;
        //println!("{}", i.v_cg_coords);
    }


    //calculate the moments and products of intertia for the combined elements
    let mut ixx: f64 = 0.0;
    let mut iyy: f64 = 0.0;
    let mut izz: f64 = 0.0;
    let mut ixy: f64 = 0.0;
    let mut ixz: f64 = 0.0;
    let mut iyz: f64 = 0.0;

    for i in rigidbod.element.iter()
    {
        ixx = ixx + i.v_local_inertia.x + i.f_mass *
            (i.v_cg_coords.y * i.v_cg_coords.y +
            i.v_cg_coords.z * i.v_cg_coords.z);

        iyy = iyy + i.v_local_inertia.y + i.f_mass *
            (i.v_cg_coords.z * i.v_cg_coords.z +
            i.v_cg_coords.x * i.v_cg_coords.x);

        izz = izz + i.v_local_inertia.z + i.f_mass *
            (i.v_cg_coords.x * i.v_cg_coords.x +
            i.v_cg_coords.y * i.v_cg_coords.y);

        ixy = ixy + i.f_mass * (i.v_cg_coords.x * 
            i.v_cg_coords.y);

        ixz = ixz + i.f_mass * (i.v_cg_coords.x * 
            i.v_cg_coords.z);
        
        iyz = iyz + i.f_mass * (i.v_cg_coords.y *
            i.v_cg_coords.z);
    }

    //finally, set up airplanes mass and inertia matrix
    rigidbod.mass = total_mass;
    //println!("{}", rigidbod.mass);

    //using nalgebra matrix
    rigidbod.m_inertia = Matrix3::new(ixx, -ixy, -ixz,
                                     -ixy, iyy, -iyz,
                                     -ixz, -iyz, izz);

                                 //println!("{}", rigidbod.m_inertia);

    //get inverse of matrix
    rigidbod.m_inertia_inverse = rigidbod.m_inertia.try_inverse().unwrap();

    //println!("{}", rigidbod.m_inertia_inverse);

    
}





//FORCES
//calculates all of the forces and moments on the plane at any time (called inside eom system)
fn calc_airplane_loads(rigidbod: &mut RigidBody)
{

    //println!("{}", "calculating forces...");

    let mut fb = Vector3::new(0.0, 0.0, 0.0);
    let mut mb = Vector3::new(0.0, 0.0, 0.0);

    //Reset forces and moments
    rigidbod.v_forces = Vector3::new(0.0, 0.0, 0.0);
    rigidbod.v_moments = Vector3::new(0.0, 0.0, 0.0);

    //Define thrust vector, which acts through the plane's center of gravity
    let mut thrust = Vector3::new(1.0, 0.0, 0.0);
    thrust = thrust * rigidbod.thrustforce; 
    //println!("{}", thrust);

    //Calculate forces and moments in body space
    let mut v_local_velocity = Vector3::new(0.0, 0.0, 0.0);
    let mut f_local_speed: f64 = 0.0;
    let mut v_drag_vector = Vector3::new(1.0, 1.0, 1.0); //VERY IMPORTANT THAT THESE WERE SET TO 1.0...
    let mut v_lift_vector = Vector3::new(1.0, 1.0, 1.0);
    let mut f_attack_angle: f64 = 0.0;
    let mut tmp: f64 = 0.0;
    let mut v_resultant= Vector3::new(0.0, 0.0, 0.0);
    let mut vtmp = Vector3::new(0.0, 0.0, 0.0);
    rigidbod.stalling = false;

    //Loop through the 7 lifting elements, skipping the fuselage
    for i in 0..8 
    {
        if i == 6 //Tail rudder. its a special case because it can rotate, so the normal vector is recalculated
        {
            let inn: f64 = (rigidbod.element[i].f_incidence).to_radians();
            let di: f64 = (rigidbod.element[i].f_dihedral).to_radians();
            rigidbod.element[i].v_normal = Vector3::new(inn.sin(), inn.cos() * di.sin(), inn.cos() * di.cos());
            rigidbod.element[i].v_normal = rigidbod.element[i].v_normal.normalize(); 
            //println!("{}", inn);
            //println!("{}", di);
        }
           // println!("{}", rigidbod.element[i].v_normal);
            
       
        //Calculate local velocity at element. This includes the velocity due to linear motion of the airplane plus the velocity and each element due to rotation
       
        //Rotation part
        vtmp = rigidbod.v_angular_velocity.cross(&rigidbod.element[i].v_cg_coords); 
        // println!("{}", rigidbod.v_angular_velocity); 
        // println!("{}", vtmp); 
        //println!("{}", rigidbod.element[i].v_cg_coords); 

        v_local_velocity = rigidbod.v_velocity_body + vtmp;
        //println!("{}", v_local_velocity);
        //println!("{}", f_local_speed);
       // println!("{}", rigidbod.v_velocity_body); 

        //Calculate local air speed
        f_local_speed = rigidbod.v_velocity_body.magnitude(); 
        //println!("{}", f_local_speed); 
         // println!("{}", rigidbod.v_velocity_body); 


        //Find the direction that drag will act. it will be in line with the relative velocity but going in the opposite direction
        if f_local_speed > 1.0
        {
            v_drag_vector = -v_local_velocity / f_local_speed;
        }
        //println!("{}", v_drag_vector);
       

        //Find direction that lift will act. lift is perpendicular to the drag vector
        //v_lift_vector = (v_drag_vector.cross(&rigidbod.element[i].v_normal)).cross(&v_drag_vector);
        let lift_tmp = v_drag_vector.cross(&rigidbod.element[i].v_normal);
        v_lift_vector = lift_tmp.cross(&v_drag_vector);
        //println!("{}", v_drag_vector);  
       // println!("{}", v_lift_vector); 


        tmp = v_lift_vector.magnitude(); 
       // tmp = (v_lift_vector.x * v_lift_vector.x + v_lift_vector.y * v_lift_vector.y + v_lift_vector.z * v_lift_vector.z).sqrt();
        //println!("{}", tmp); 

        v_lift_vector = v_lift_vector.normalize(); //THIS WAS ONE LINE MESSING EVERYTHIGN UP //////////////////////////!!!!!!!!!!!!!!!!!!!!!!
        //println!("{}", v_lift_vector);
  
        //FINDING NORMALIZED VECTOR BY HAND BECAUSE FOR SOME REASON THE ABOVE LINE SPITS OUT NAN
        // let tol:f64 = 0.0001;
        // let mut m: f64 = (v_lift_vector.x * v_lift_vector.x + v_lift_vector.y * v_lift_vector.y + v_lift_vector.z * v_lift_vector.z).sqrt();
        // if m <= tol
        // {
        //      m = 1.0;
        // }
        // v_lift_vector.x /= m;
        // v_lift_vector.y /= m;
        // v_lift_vector.z /= m;
        // if v_lift_vector.x.abs() < tol 
        // {
        //     v_lift_vector.x = 0.0;
        // }
        // if v_lift_vector.y.abs() < tol
        // { 
        //     v_lift_vector.y = 0.0;
        // }
        // if v_lift_vector.z.abs() < tol
        // {
        //     v_lift_vector.z = 0.0;
        // }
        // println!("{}", v_lift_vector);

        //Find the angle of attack. its the angle between the lift vector and element normal vector 
        tmp = v_drag_vector.dot(&rigidbod.element[i].v_normal);
       // println!("{}", tmp);
        if tmp > 1.0
        {
            tmp = 1.0;
        }
        if tmp < -1.0
        {
            tmp = -1.0;
        }

        //f_attack_angle = (tmp.asin()).to_radians();
        f_attack_angle = tmp.asin(); //asin gives in radians...
        // println!("{}", f_attack_angle);

        //Determine lift and drag force on the element (rho is 1.225 in the book)  BUT IN THE COMMON HEADER FILE IT IS 0.0023769
        //but using 0.00237 it doesnt respond at all...
        //println!("{}", f_local_speed); 
        tmp = 0.5 * 1.225 * f_local_speed * f_local_speed * rigidbod.element[i].f_area;
        //println!("{}", tmp); 
        //println!("{}", v_resultant);
        

        if i == 6 //tail/ rudder
        {
            v_resultant = (v_lift_vector * rudder_lift_coefficient(f_attack_angle) + v_drag_vector * rudder_drag_coefficient(f_attack_angle)) * tmp;
        }
        //this is not in the book code but its in the actual code...
        // else if i == 7
        // {
        //     v_resultant = v_drag_vector * 0.5 * tmp;
        // }
        else
        {
            v_resultant = (v_lift_vector * lift_coefficient(f_attack_angle, rigidbod.element[i].i_flap) + v_drag_vector * drag_coefficient(f_attack_angle, rigidbod.element[i].i_flap)) * tmp;
        }
        // println!("lift {}", v_lift_vector);
        // println!("drag {}", v_drag_vector);
        // println!("flap {}", rigidbod.element[i].i_flap);
        // println!("atk angle {}", f_attack_angle);
       // println!("resultant {}", v_resultant);

        //Check for stall. if the coefficient of lift is 0, stall is occuring.
        //this is how the book code does it
        // if lift_coefficient(f_attack_angle, rigidbod.element[i].i_flap) == 0.0
        // {
        //     rigidbod.stalling = true; 
        // }

        //this is how actual code does it
        if i <= 3
        {
            if lift_coefficient(f_attack_angle, rigidbod.element[i].i_flap) == 0.0
            {
                rigidbod.stalling = true; 
            }
        }

        //Keep running total of resultant forces (total force)
        fb = fb + v_resultant;

        //println!("{}", fb);

        //Calculate the moment about the center of gravity of this element's force and keep them in a running total of these moments (total moment)
        vtmp = rigidbod.element[i].v_cg_coords.cross(&v_resultant);
        //println!("{}", rigidbod.element[i].v_cg_coords); 
        //println!("{}", vtmp); 
        //println!("{}", v_resultant); 

        mb = mb + vtmp;
        //println!("{}", mb);

     }

    //Add thrust
    fb = fb + thrust;
    //println!("{}", fb);

    //DID THIS FIRST
    //Convert forces from model space to earth space. rotates the vector by the unit quaternion (QVRotate function)
    //(the first try and next try have similar results in flightgear floatign around)
    //rigidbod.v_forces = rigidbod.q_orientation_unit.transform_vector(&fb); 
    //println!("{}", rigidbod.q_orientation_unit);
    //println!("{}", rigidbod.v_forces);

    //THEN TRIED THIS SECOND
    //Doing QVRotate by hand w/o using unit quaternion. I cannot multiply fb by the quaternion so i will put fb into a quaternion first and then multiply
    //Doing all this because we arent doing stuff with unit quaternion yet
    //quat*quat
    let fbtmp =  Quaternion::new(0.0, fb.x, fb.y, fb.z);   //make a quaternion with scalar 0
    let quatmp = rigidbod.q_orientation * fbtmp * rigidbod.q_orientation.conjugate();
    let vectmp = quatmp.vector();
    rigidbod.v_forces = Vector3::new(vectmp[0], vectmp[1], vectmp[2]);  // Recall that the quaternion is stored internally as (i, j, k, w)
                                                                        // while the crate::new constructor takes the arguments as (w, i, j, k).
    //OR vec*vec ..works like above...
    // let qvec = rigidbod.q_orientation.vector(); //make vec out of quaternion (dont need scalar)
    // let qvec3 = Vector3::new(qvec[0], qvec[1], qvec[2]); //populate nalgebra vec with the values
    // let qvec3_conj = Vector3::new(-qvec[0], -qvec[1], -qvec[2]);
    // rigidbod.v_forces = (qvec3.cross(&fb)).cross(&qvec3_conj);


    // //NOW TRY THIS THIRD
    // //convert q orientatino to a unit quaternion in order to do the transform_vector call
    //let quat_to_unit = UnitQuaternion::from_quaternion(rigidbod.q_orientation); 
    //rigidbod.v_forces = quat_to_unit.transform_vector(&fb);

    //TRY FOURTH
    //TAKE UNIT QUATERNION, MAKE QUATERNION OUT OF IT (WITH SCALAR 0), AND THEN MAKE QUATERNION OUT OF FB IN ORDER TO DO QVROATE BY HAND FOR VFORCES
    //idea is that i need to stay consistent with when i am using quaternion vs unit quaternion



    //apply gravity (g is -32.174 ft/s^2), ONLY APPLY WHEN ALTITUDE IS GREATER THAN ZERO.... how to fidn altitude????

    //if rigidbod.alt > 0.0
    //{
        rigidbod.v_forces.z = rigidbod.v_forces.z + (-32.174) * rigidbod.mass;
    //}
    
   // println!("{}", rigidbod.v_forces.z);

    rigidbod.v_moments = rigidbod.v_moments + mb;
    //println!("{}", rigidbod.v_moments);


    //  println!("drag {}" ,v_drag_vector);
    //  println!("lift {}", v_lift_vector);
    // println!("{}", f_local_speed);


}




//System to perform physics calculations based on forces "stepsimulation"
struct EquationsOfMotion;
impl<'a> System<'a> for EquationsOfMotion
{
    type SystemData = (
        WriteStorage<'a, RigidBody>,
        ReadStorage<'a, KeyboardState>
    );

    fn run(&mut self, (mut rigidbody, keyboardstate): Self::SystemData) 
    {
        for (mut rigidbod, keystate) in (&mut rigidbody, &keyboardstate).join() 
        {

            //println!("{}", rigidbod.q_orientation_unit);
           // println!("{}", "inside eom");
            let max_thrust = 3000.0; //max thrustforce value //was 3000
            let d_thrust = 100.0;   //change in thrust per keypress //was 100



            //reset/zero the elevators, rudders, and ailerons
            //rudder
            rigidbod.element[6].f_incidence = 0.0;
            //ailerons
            rigidbod.element[0].i_flap = 0;
            rigidbod.element[3].i_flap = 0;
            //elevators
            rigidbod.element[4].i_flap = 0;
            rigidbod.element[5].i_flap = 0;


            //Handle the input states
            //Thrust states
            if rigidbod.thrustforce < max_thrust && keystate.thrust_up == true
            {
                rigidbod.thrustforce = rigidbod.thrustforce + d_thrust;
                
            }   
            else if rigidbod.thrustforce > 0.0 && keystate.thrust_down == true
            {
                rigidbod.thrustforce = rigidbod.thrustforce - d_thrust;

            } 

            //Rudder States
            if keystate.left_rudder == true
            { 
                rigidbod.element[6].f_incidence = 16.0;
            } 
            else if keystate.right_rudder == true
            { 
                rigidbod.element[6].f_incidence = -16.0;
            } 
            // else //if keystate.zero_rudder == true
            // { 
            //     rigidbod.element[6].f_incidence = 0.0;
            // }

            //Roll States
            if keystate.roll_left == true
            { 
                rigidbod.element[0].i_flap = 1;
                rigidbod.element[3].i_flap = -1;
            } 
            else if keystate.roll_right == true
            { 
                rigidbod.element[0].i_flap = -1;
                rigidbod.element[3].i_flap = 1;
            } 
            // else //if keystate.zero_ailerons == true
            // { 
            //     rigidbod.element[0].i_flap = 0;
            //     rigidbod.element[3].i_flap = 0;
            // }

            //Pitch States
            if keystate.pitch_up == true
            { 
                rigidbod.element[4].i_flap = 1;
                rigidbod.element[5].i_flap = 1;
                //println!("{}", "PITCHING UP");
            } 
            else if keystate.pitch_down == true
            { 
                rigidbod.element[4].i_flap = -1;
                rigidbod.element[5].i_flap = -1;

            } 
            // else //if keystate.zero_elevators == true
            // { 
            //     rigidbod.element[4].i_flap = 0;
            //     rigidbod.element[5].i_flap = 0;
            //}

            //Flap States
            if keystate.flaps_down == true
            { 
                rigidbod.element[1].i_flap = -1;
                rigidbod.element[2].i_flap = -1;
                rigidbod.flaps = true;
            } 
            else if keystate.zero_flaps == true //this is not the same as the zeroing done in the beggining each loop
            { 
                rigidbod.element[1].i_flap = 0;
                rigidbod.element[2].i_flap = 0;
            } 


            //begin the step simulation part
    
            //--------------------Calculate all of the forces and moments on the airplane
            calc_airplane_loads(&mut rigidbod);

            //println!("{:#?}", rigidbod);


            //--------------------Calculate acceleration of airplane in earth space
            let ae: Vector3<f64> = rigidbod.v_forces / rigidbod.mass;
            //println!("{}", rigidbod.v_forces);

         //println!("{}", ae);
  
            //println!("{}", rigidbod.mass);



            //----------------Calculate velocity of airplane in earth space
           rigidbod.v_velocity += ae * DT; 
          // rigidbod.v_velocity = rigidbod.v_velocity + Vector3::new(0.0, 0.0, 50.0);
            //println!("{}", rigidbod.v_velocity.y); 

           // println!("{}", rigidbod.v_velocity.normalize()); 
    

            //---------------Calculate position of airplane in earth space
            rigidbod.v_position = rigidbod.v_position + rigidbod.v_velocity * DT;

            //rigidbod.v_position_lla = rigidbod.v_velocity.normalize(); //trying to see a normalized velocity vector for direction...
            //println!("{}", rigidbod.v_velocity);
        //     rigidbod.v_position_lla = geo::ecef2lla(&rigidbod.v_position, &ELLIPSOID); //immediately convert to lla

        //     if rigidbod.v_position_lla.z < 0.0
        //     {
        //         rigidbod.v_position_lla.z = 0.0;
        //     }

        //    // rigidbod.alt = rigidbod.alt + (rigidbod.v_position_lla.z - 0.0); //find altitude by finding difference in ecef (try dif in lla if dont work)
        //     //println!("{}", rigidbod.alt);

        //     //let lla_temp_vec = Vector3::new(rigidbod.v_position_lla.x, rigidbod.v_position_lla.y, lla.z); //make vector with lla values to convert back to ecef

        //     rigidbod.v_position = geo::lla2ecef(&rigidbod.v_position_lla, &ELLIPSOID);





            //GRAVITY, HEADING, ALTITUDE issues. always find ecef based on 0 altitude so take the new ecef with that new x val or y val
            //need to find where altitude is calculated and could just use that like how 2d does it

           // println!("{}", rigidbod.v_position);
    
            //Now handle rotations:
            //let mut mag: f64 = 0.0;
    
            //Calculate angular velocity of airplane in body space
            //TEST

            rigidbod.v_angular_velocity = rigidbod.v_angular_velocity + rigidbod.m_inertia_inverse * ( rigidbod.v_moments - ( rigidbod.v_angular_velocity.cross(&(rigidbod.m_inertia * rigidbod.v_angular_velocity)))) * DT;
            //println!("{}", rigidbod.v_angular_velocity); 
            // println!("{}", rigidbod.v_moments);
            // println!("{}", rigidbod.m_inertia_inverse);
            // println!("{}", rigidbod.m_inertia);

    


            //-----------------Calculate the new rotation quaternion
            //we need angular velocity to be in a quaternion form for the multiplication.... ( i think this gives an accurate result...) because 
            //nalgebra wont let me multiply the vector3 of angular velocity with the unit quaternion ( or a regular quaternion)
            //so im going to create Quaternion based on the angular velocity ( i hope this math works out properly given the work around with nalgbra...) if not ill have to do it by hand
            //I think this is correct and ok because Bourgs book has a vector * quaternion function: "This operator multiplies the quaternion q by the vector v as though the vector v were a quaternion with its scalar component equal to 0.


            //TRIED THIS FIRST
            //making the quaternion based on angular velocity and scalar as 0 (because we cant just multiply quaternion by vector, but we can multiply quaternion by quaternion)
            let qtmp =  Quaternion::new(0.0, rigidbod.v_angular_velocity.x, rigidbod.v_angular_velocity.y, rigidbod.v_angular_velocity.z);                    
            rigidbod.q_orientation = rigidbod.q_orientation + (rigidbod.q_orientation * qtmp) * (0.5 * DT); 
            //println!("{}", qtmp);
            //println!("{}", rigidbod.q_orientation); 

            //TRY THIS SECOND
            //make quaternion from the unit quaternion
           ////////////let unitq_to_q = UnitQuaternion::quaternion(&rigidbod.q_orientation_unit);
            //make a quaternion based on angular velocity (scalar is 0)
           /////////// let ang_q_tmp =  Quaternion::new(0.0, rigidbod.v_angular_velocity.x, rigidbod.v_angular_velocity.y, rigidbod.v_angular_velocity.z);                    
            //do the operations for new rotation quaternion based on angular velocity
           ////////////let new_unitq_to_q = unitq_to_q + (unitq_to_q * ang_q_tmp) * (0.5 * DT); 
            //println!("{}", unitq_to_q); 
            //println!("{}", ang_q_tmp);
            //println!("{}", new_unitq_to_q);


            //TRY THIS THIRD OCT 8TH
            //make current quaternion into vector to multiply with angular veloocity
            // let q_as_vec0 = rigidbod.q_orientation.vector();
            // let mut q_as_vec = Vector3::new(q_as_vec0[0], q_as_vec0[1], q_as_vec0[2]);
            // //now we have quaternion as a vector, so multiply it with angular velocity
            // q_as_vec = q_as_vec + (q_as_vec.cross( &rigidbod.v_angular_velocity)) * (0.5 * DT);
            // rigidbod.q_orientation =  Quaternion::new(1.0, q_as_vec.x, q_as_vec.y, q_as_vec.z);                  
            
            //TRY FOURTH OCT 8
            //multiply unit vec based on angular velocity by the unit vec (THIS DOESNT WANT TO WORK CUZ CANT MULTIPLY UNIT VEC BY F64)
            // let q_unit_tmp =  UnitQuaternion::new(Vector3::new(rigidbod.v_angular_velocity.x, rigidbod.v_angular_velocity.y, rigidbod.v_angular_velocity.z));                    
            // rigidbod.q_orientation_unit = rigidbod.q_orientation_unit + (rigidbod.q_orientation_unit * q_unit_tmp) * (0.5 * DT); 



            //----------------Now normalize the orientation quaternion (make into unit quaternion)
            //problem is that after we normalize (make unit q)m we have to go strtaight into calculating
            //the velocity in body sdpace which requires qvrotate which multiplies quaternions and 
            
            //TRIED THIS FIRST
            //rigidbod.q_orientation_unit = UnitQuaternion::new_normalize(rigidbod.q_orientation);
            // println!("{}", rigidbod.q_orientation_unit);

            //TRYING THIS SECOND
            //take quaternion created above to the new unit quaternion (input gets normalized)
            /////////////rigidbod.q_orientation_unit = UnitQuaternion::from_quaternion(new_unitq_to_q);
            //println!("{}", rigidbod.q_orientation_unit);

            //TRYING THIS THIRD (basially same as first)
            //take the quaternion that we track and make it into unit. 
            rigidbod.q_orientation_unit = UnitQuaternion::from_quaternion(rigidbod.q_orientation);
            
            //TRYING THIS FOURTH
            //dont actually make a unit quaternion... just normalize the regular one
            //rigidbod.q_orientation = rigidbod.q_orientation.normalize();

            //fifth oct 27
            // let mag = rigidbod.q_orientation.magnitude();
            // if mag != 0.0
            // {
            //    rigidbod.q_orientation /= mag;
            // }




            //---------------------Calculate the velocity in body space
            //TRYING FIRST TO USE THE VECTOR TRANSFORM
            //rigidbod.v_velocity_body = (rigidbod.q_orientation_unit.conjugate()).transform_vector(&rigidbod.v_velocity);
            //println!("{}", rigidbod.v_velocity_body); 
            //GOT RID OF GRAVITY ADDITION IN CAL LOADS FOR NOW

            //SECOND TRYING TO DO QVROTATE BY HAND WITH REGULAR QUATERNION
            // let veltmp =  Quaternion::new(0.0, rigidbod.v_velocity.x, rigidbod.v_velocity.y, rigidbod.v_velocity.z);   
            // let conjtemp = rigidbod.q_orientation.conjugate();
            // let quatmp = conjtemp * veltmp * conjtemp.conjugate();
            // let vectmp = quatmp.vector();
            // rigidbod.v_velocity_body = Vector3::new(vectmp[0], vectmp[1], vectmp[2]);

            //THRID CREATE QUATERNION FROM NORMALIZED UNIT AND THEN DO QVR BY HAND
            //  let unitq_to_q = UnitQuaternion::quaternion(&rigidbod.q_orientation_unit); //create regular quaternion
            //  let veltmp =  Quaternion::new(0.0, rigidbod.v_velocity.x, rigidbod.v_velocity.y, rigidbod.v_velocity.z);   
            // let conjtemp = unitq_to_q.conjugate();
            //  let quatmp = conjtemp * veltmp * conjtemp.conjugate();
            // let vectmp = quatmp.vector();
            //  rigidbod.v_velocity_body = Vector3::new(vectmp[0], vectmp[1], vectmp[2]);

            //FOURTH.... use the unit q to do qvrotate by hand
           let vel_unit_tmp =  UnitQuaternion::new(Vector3::new(rigidbod.v_velocity.x, rigidbod.v_velocity.y, rigidbod.v_velocity.z));
           let conjtemp = rigidbod.q_orientation_unit.conjugate();
           let quatmp = conjtemp * vel_unit_tmp * rigidbod.q_orientation_unit.conjugate();
           let vectmp = quatmp.vector();
           rigidbod.v_velocity_body = Vector3::new(vectmp[0], vectmp[1], vectmp[2]);

            //fifth oct 27
            // let veltmp =  Quaternion::new(0.0, rigidbod.v_velocity.x, rigidbod.v_velocity.y, rigidbod.v_velocity.z);   
            // let conjtemp = rigidbod.q_orientation.conjugate();
            // let quatmp = conjtemp * veltmp * conjtemp.conjugate();
            // let vectmp = quatmp.vector();
            // rigidbod.v_velocity_body = Vector3::new(vectmp[0], vectmp[1], vectmp[2]);

    
            //---------------calculate air speed
            rigidbod.f_speed = rigidbod.v_velocity.magnitude(); 
            // rigidbod.f_speed = (rigidbod.v_velocity.x * rigidbod.v_velocity.x + rigidbod.v_velocity.y * rigidbod.v_velocity.y + rigidbod.v_velocity.z * rigidbod.v_velocity.z).sqrt();
             //println!("{}", rigidbod.f_speed); 
    
           // get euler angles for our info
            let euler = rigidbod.q_orientation_unit.euler_angles();
            rigidbod.v_euler_angles.x = euler.0; //roll
            rigidbod.v_euler_angles.y = euler.1; //pitch
            rigidbod.v_euler_angles.z = euler.2; //yaw
           // println!("{:?}", euler); 
    
            
            rigidbod.frame_count = rigidbod.frame_count + 1.0;

        }//end for
    }//end run
}//end system


//System to send packets
struct SendPacket;
impl<'a> System<'a> for SendPacket
{
    type SystemData = (
            ReadStorage<'a, RigidBody>,
            ReadStorage<'a, FGNetFDM>,
    );

    fn run(&mut self, (rigidbody, fgnetfdm): Self::SystemData) 
    {
        for (rigidbod, _netfdm,) in (&rigidbody, &fgnetfdm).join() 
        {
                                    //lat, lon
            //ktts airport location: 28.5971, -80.6827, 609.6 meters = 2000 ft

            //println!("{}", "inside send packet");


            //All data passed into the FGNetFDM struct is converted to network byte order

            //Create fdm instance
            let mut fdm: FGNetFDM = Default::default();
            
            //Set Roll, Pitch, Yaw
           let roll: f32 = rigidbod.v_euler_angles.x as f32;
           let pitch: f32 =  rigidbod.v_euler_angles.y as f32; 
           let yaw: f32 =  (rigidbod.v_euler_angles.z as f32).to_degrees(); 

            //Coordinate conversion: cartesian to geodetic
            let lla = geo::ecef2lla(&rigidbod.v_position, &ELLIPSOID); 

            //Set lat, long, alt
           // println!("{}", lla * 0.3048);
            fdm.latitude = f64::from_be_bytes(lla.x.to_ne_bytes());
            fdm.longitude = f64::from_be_bytes(lla.y.to_ne_bytes()); 
            fdm.altitude = f64::from_be_bytes(lla.z.to_ne_bytes()); //lla.z seems to increase altitude artificially... //rigidbod.alt
           
            //Roll, Pitch, Yaw
            fdm.phi = f32::from_be_bytes(roll.to_ne_bytes());
            fdm.theta = f32::from_be_bytes(pitch.to_ne_bytes()); 
            fdm.psi = f32::from_be_bytes(yaw.to_radians().to_ne_bytes());

            //Other airplane data
            let fg_net_fdm_version = 24_u32;
            let visibility: f32 = 5000.0;
            fdm.version = u32::from_be_bytes(fg_net_fdm_version.to_ne_bytes());
            fdm.num_engines = u32::from_be_bytes(1_u32.to_ne_bytes());
            fdm.num_tanks = u32::from_be_bytes(1_u32.to_ne_bytes());
            fdm.num_wheels = u32::from_be_bytes(1_u32.to_ne_bytes());
            fdm.warp = f32::from_be_bytes(1_f32.to_ne_bytes());
            fdm.visibility = f32::from_be_bytes(visibility.to_ne_bytes());


            //Convert struct array of u8 of bytes
            let bytes: &[u8] = unsafe { any_as_u8_slice(&fdm) };

            //Finally send &[u8] of bytes to flight gear
            //Connect first (would be nice to only do this once...)
            SOCKET.connect("127.0.0.1:5500").expect("connect function failed");
            //Send!
            SOCKET.send(bytes).expect("couldn't send message");



            //Print some relevant data
            disable_raw_mode().unwrap(); //Get out of raw mode to print clearly
            //println!("{:#?}", rigidbod);
            println!("{}", "--------------------------------------------------");
            println!("position ecef: {:?}", rigidbod.v_position);
            println!("position lla: {:?}", lla);
            //println!("position delta {:?}", rigidbod.v_position_lla);
            println!("velocity: {:?}", rigidbod.v_velocity);
            println!("velocity body: {:?}", rigidbod.v_velocity_body);
            println!("angular velocity: {:?}", rigidbod.v_angular_velocity);
            //println!("euler angles: {:?}", rigidbod.v_euler_angles);
           println!("euler anglesx: {:?}", rigidbod.v_euler_angles.x.to_degrees());
           println!("euler anglesy: {:?}", rigidbod.v_euler_angles.y.to_degrees());
           println!("euler anglesz: {:?}", rigidbod.v_euler_angles.z.to_degrees());
            println!("speed (knots): {:?}", rigidbod.f_speed/1.688 );
            println!("unit quaternion: {:?}", rigidbod.q_orientation_unit);
            println!("quaternion: {:?}", rigidbod.q_orientation);
            println!("forces: {:?}", rigidbod.v_forces);
            println!("moments: {:?}", rigidbod.v_moments);
            println!("altitude {}", lla.z);// rigidbod.alt);
            println!("time: {}", rigidbod.frame_count * DT);// rigidbod.alt);
           // println!("lla.z{}", lla.z );
            
            //println!("time = {}", outdata.s);
            //println!("x traveled (m) = {}", outdata.q[1] / 3.6); //converted to meters
            //println!("{}", rigidbod.v_position);
           // println!("altitude (m) = {}", lla.z);
            //println!("airspeed (km/h) = {}", outdata.airspeed);
            //println!("throttle % = {}", inpdata.throttle);
           // println!("angle of attack (deg) = {}", inpdata.alpha);
            //println!("x travel change (m) since last frame = {}", outdata.delta_traveled);
            //println!("bank angle (deg) = {}", inpdata.bank);
            //println!("y = {}", outdata.q[3]);
            enable_raw_mode().unwrap(); //Return to raw

  

        }//end for
    }//end run
}//end system


async fn handle_input(thrust_up: &mut bool, thrust_down: &mut bool, left_rudder: &mut bool, right_rudder: &mut bool, roll_left: &mut bool, roll_right: &mut bool, pitch_up: &mut bool, pitch_down: &mut bool, flaps_down: &mut bool, zero_flaps: &mut bool) 
{
    let mut reader = EventStream::new();
    let mut delay = Delay::new(Duration::from_millis(2)).fuse(); 
    let mut event = reader.next().fuse();

    //Either the time delay or keyboard event happens and then this function will be called again in the system
    select!
    {
        _ = delay => 
        { 
            return; //Exit if time expires
        }, 

        maybe_event = event =>
        {
            match maybe_event 
            {
                Some(Ok(event)) => 
                {
                    println!("Event::{:?}\r", event);

                    //Thrust
                    if event == Event::Key(KeyCode::Char('a').into()) 
                    {
                        *thrust_up = true;
                    }
                    else if event == Event::Key(KeyCode::Char('z').into()) 
                    {
                        *thrust_down = true;
                    }

                    //Rudders for yaw
                    else if event == Event::Key(KeyCode::Char('n').into()) 
                    {
                        *left_rudder = true;
                    }
                    else if event == Event::Key(KeyCode::Char('m').into()) 
                    {
                        *right_rudder = true;
                    }
                    // else if event == Event::Key(KeyCode::Char('d').into()) 
                    // {
                    //     *zero_rudder = true;
                    // }

                    //Ailerons for roll
                    else if event == Event::Key(KeyCode::Left.into()) 
                    {
                        *roll_left = true;
                    }
                    else if event == Event::Key(KeyCode::Right.into()) 
                    {
                        *roll_right = true;
                    }
                    // else if event == Event::Key(KeyCode::Char('g').into()) 
                    // {
                    //     *zero_ailerons = true;
                    // }


                    //Elevators for Pitch
                    else if event == Event::Key(KeyCode::Up.into()) 
                    {
                        *pitch_up = true;
                    }
                    else if event == Event::Key(KeyCode::Down.into()) 
                    {
                        *pitch_down = true;
                    }
                    // else if event == Event::Key(KeyCode::Char('h').into()) 
                    // {
                    //     *zero_elevators = true;
                    // }

                    //Flaps for lift
                    else if event == Event::Key(KeyCode::Char('f').into()) 
                    {
                        *flaps_down = true;
                        *zero_flaps = false;
                    }
                    else if event == Event::Key(KeyCode::Char('g').into()) 
                    {
                        *zero_flaps = true;
                        *flaps_down = false;
                    }

                    //Quit program
                    else if event == Event::Key(KeyCode::Char('q').into()) 
                    {
                        //Exit program... maybe a better way to do this?
                        disable_raw_mode().unwrap();
                        process::exit(1);
                    }


                }
                Some(Err(e)) => println!("Error: {:?}\r", e),

                None => return,
            }
        },
    };
}

//System to handle user input
struct FlightControl;
impl<'a> System<'a> for FlightControl
{
    type SystemData = ( 
        ReadStorage<'a, RigidBody>, 
        WriteStorage<'a, KeyboardState>,
    );

    fn run(&mut self, (rigidbody, mut keyboardstate): Self::SystemData) 
    {
        for (_rigidbod, keystate) in (&rigidbody, &mut keyboardstate).join() 
        {
            //println!("{}", "inside flight control");
            //Set all states false before we know if they are being activated
            keystate.thrust_up = false; 
            keystate.thrust_down = false;

            keystate.left_rudder = false;
            keystate.right_rudder = false;
            //keystate.zero_rudder = true,
        
            keystate.roll_left = false;
            keystate.roll_right = false;
            //keystate.zero_ailerons = true,
        
            keystate.pitch_up = false;
            keystate.pitch_down = false;
            //keystate.zero_elevators = true,
        
            //flaps will be toggled on and off so do not set them false for every iteration
            // keystate.flaps_down = false;
            // keystate.zero_flaps = false;


            //Enter raw mode for terminal input
            enable_raw_mode().unwrap();

            let mut stdout = stdout();

            //This will make output not as crazy
            execute!(stdout, Clear(ClearType::All), cursor::MoveTo(0, 0)) .unwrap();
           
            //Handle flight control
            async_std::task::block_on(handle_input(&mut keystate.thrust_up, &mut keystate.thrust_down, &mut keystate.left_rudder, &mut keystate.right_rudder, &mut keystate.roll_left, &mut keystate.roll_right, &mut keystate.pitch_up,&mut keystate.pitch_down, &mut keystate.flaps_down, &mut keystate.zero_flaps));
        
            disable_raw_mode().unwrap();

        }//end for
    }//end run
}//end system


//create dummy plane to calculate mass properties






//Set some global variables:

//Macro to define other globals
lazy_static!
{
    //define earth ellipsoid
    static ref ELLIPSOID: coord_transforms::structs::geo_ellipsoid::geo_ellipsoid = geo_ellipsoid::geo_ellipsoid::new(geo_ellipsoid::WGS84_SEMI_MAJOR_AXIS_METERS, geo_ellipsoid::WGS84_FLATTENING);
    //create socket
    static ref SOCKET: std::net::UdpSocket = UdpSocket::bind("127.0.0.1:1337").expect("couldn't bind to address");

}

//Time in between each eom calculation
static DT: f64 = 0.033;//0.016;

fn main()
{
    //Create world
    let mut world = World::new();
    world.register::<RigidBody>();
    world.register::<FGNetFDM>();
    world.register::<KeyboardState>();

    //Create dispatcher of the systems
    let mut dispatcher = DispatcherBuilder::new()
    .with(FlightControl, "flightcontrol", &[])
    .with(EquationsOfMotion, "EOM", &[])
    .with(SendPacket, "sendpacket", &["EOM"])
    .build();
    dispatcher.setup(&mut world);




    //Intialize the airplane
    let mut myairplane = RigidBody{
        mass: 0.0,
        m_inertia: Matrix3::new(0.0, 0.0, 0.0,
                                0.0, 0.0, 0.0,
                                0.0, 0.0, 0.0),
        m_inertia_inverse: Matrix3::new(0.0, 0.0, 0.0,
                                        0.0, 0.0, 0.0,
                                        0.0, 0.0, 0.0),
        v_position: Vector3::new(907440.867577218, -5530938.88177552, 3035061.57686847),                
         // ktts flat (907354.212367197, -5530410.70998214, 3034769.79340209)
         // ktts at 100m above ground (907368.427460705,-5530497.3523367, 3034817.65814395 ),
        //c++ start position (believe distances are in feet)... (-5000.0, 0.0, 2000.0), 
        // ktts at 2000 ft above (609.6 meters) 907440.867577218, -5530938.88177552, 3035061.57686847
        v_velocity: Vector3::new(60.0, 0.0, 0.0),        //set initial velocity //was 60 in x...
        v_euler_angles: Vector3::new(0.0, 0.0, 0.0), //not defined in book here
        f_speed: 60.0, //was 60..
        v_angular_velocity: Vector3::new(0.0, 0.0, 0.0),        //set angular velocity
        v_forces: Vector3::new(500.0, 0.0, 0.0),        //set initial thrust, forces, and moments //was 500 in x
        thrustforce: 500.0,   //this isnt written in the rigid body intiialization for some reason...
        v_moments: Vector3::new(0.0, 0.0, 0.0),
        v_velocity_body: Vector3::new(0.0, 0.0, 0.0),        //zero the velocity in body space coordinates
        //set these to false at first, will control later with keyboard... these are not defined in the structure
        stalling: false,
        flaps: false,
        q_orientation: Quaternion::new(1.0, 0.0, 0.0, 0.0), //start with something
        q_orientation_unit: UnitQuaternion::identity(),//UnitQuaternion::from_euler_angles(0.0, 0.0, 0.0), //UnitQuaternion::new_normalize(Quaternion::new(0.0, 0.0, 0.0, 0.0)),
        element: vec![
            PointMass{f_mass: 6.56, v_d_coords: Vector3::new(14.5, 12.0, 2.5), v_local_inertia: Vector3::new(13.92, 10.50, 24.00), f_incidence: -3.5, f_dihedral: 0.0, f_area: 31.2, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) },
            PointMass{f_mass: 7.31, v_d_coords: Vector3::new(14.5, 5.5, 2.5), v_local_inertia: Vector3::new(21.95, 12.22, 33.67), f_incidence: -3.5, f_dihedral: 0.0, f_area: 36.4, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) },
            PointMass{f_mass: 7.31, v_d_coords: Vector3::new(14.5, -5.5, 2.5), v_local_inertia: Vector3::new(21.95, 12.22, 33.67), f_incidence: -3.5, f_dihedral: 0.0, f_area: 36.4, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) },
            PointMass{f_mass: 6.56, v_d_coords: Vector3::new(14.5, -12.0, 2.5), v_local_inertia: Vector3::new(13.92, 10.50, 24.00), f_incidence: -3.5, f_dihedral: 0.0, f_area: 31.2, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) },
            PointMass{f_mass: 2.62, v_d_coords: Vector3::new(3.03, 2.5, 3.0), v_local_inertia: Vector3::new(0.837, 0.385, 1.206), f_incidence: 0.0, f_dihedral: 0.0, f_area: 10.8, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) },
            PointMass{f_mass: 2.62, v_d_coords: Vector3::new(3.03, -2.5, 3.0), v_local_inertia: Vector3::new(0.837, 0.385, 1.206), f_incidence: 0.0, f_dihedral: 0.0, f_area: 10.8, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) },
            PointMass{f_mass: 2.93, v_d_coords: Vector3::new(2.25, 0.0, 5.0), v_local_inertia: Vector3::new(1.262, 1.942, 0.718), f_incidence: 0.0, f_dihedral: 90.0, f_area: 12.0, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) },
            PointMass{f_mass: 31.8, v_d_coords: Vector3::new(15.25, 0.0, 1.5), v_local_inertia: Vector3::new(66.30, 861.9, 861.9), f_incidence: 0.0, f_dihedral: 0.0, f_area: 84.0, i_flap: 0, v_normal: Vector3::new(0.0, 0.0, 0.0), v_cg_coords: Vector3::new(0.0, 0.0, 0.0) }
            ],
        v_position_lla: Vector3::new(0.0, 0.0, 0.0),
        frame_count: 0.0,
        alt: 0.0
    };

    //Calculate mass properties on this airplane initialized
    calc_airplane_mass_properties(&mut myairplane);



    //Create plane Entity and populate the Component data using the adata from myairplane (had to do this because i could not calculate mass properties from the entity directly)
    let _plane = world.create_entity()
    .with(RigidBody{
        mass: myairplane.mass,
        m_inertia: myairplane.m_inertia,
        m_inertia_inverse: myairplane.m_inertia_inverse,
        v_position: myairplane.v_position,        //set initial position
        v_velocity: myairplane.v_velocity,        //set initial velocity
        v_euler_angles: myairplane.v_euler_angles, //not defined in book here
        f_speed: myairplane.f_speed,
        v_angular_velocity: myairplane.v_angular_velocity,        //set angular velocity
        v_forces: myairplane.v_forces,        //set initial thrust, forces, and moments
        thrustforce: myairplane.thrustforce,   //this isnt written in the rigid body intiialization for some reason...
        v_moments: myairplane.v_moments,
        v_velocity_body: myairplane.v_velocity_body,        //zero the velocity in body space coordinates
        //set these to false at first, will control later with keyboard... these are not defined in the structure
        stalling: false,
        flaps: false,
        q_orientation: myairplane.q_orientation, 
        q_orientation_unit: myairplane.q_orientation_unit,
        element: myairplane.element,
        v_position_lla: myairplane.v_position_lla,
        alt: myairplane.alt,
        frame_count: myairplane.frame_count
    })
    .with(KeyboardState{
        //may need to delete the zero states...
        thrust_up: false,
        thrust_down: false,
    
        left_rudder: false,
        right_rudder: false,
        zero_rudder: false,
    
        roll_left: false,
        roll_right: false,
        zero_ailerons: false,
    
        pitch_up: false,
        pitch_down: false,
        zero_elevators: false,
    
        flaps_down: false,
        zero_flaps: false,
    })
    .with(FGNetFDM{
        ..Default::default()
        })
    .build();


  


    let steptime = 33.milliseconds(); //30 fps
    loop 
    {

        let start = Instant::now();

        dispatcher.dispatch(&world);
        world.maintain();


             //Create frame_rate loop
            let calc_time = start.elapsed(); //how long this calculation took

            if calc_time < steptime //if calc time takes less than 33 ms
            {

                //this was much more complicated than it should have been....
                let calc_time2 = calc_time.as_secs_f64() * 1000.0; //get the value in ms of how long the calculation ran 
                let sleep_time = steptime - calc_time2.milliseconds(); //subtract step time and the calculation time
                //steptime.checked_sub(sleep_time1.milliseconds()); //step time - sleep time
                //thread::sleep(runtime - sleep_time); //sleep for the extra time
                let sleep_time2 = sleep_time.as_seconds_f64() * 1000.0; //get value in ms 

                let sleep_duration = Duration::from_millis(sleep_time2 as u64);

                thread::sleep(sleep_duration); //sleep for the extra time

                //println!("calc time {:?}", calc_time);
                //println!("calc time 2 {:?}", calc_time2);
               // println!("sleep time {:?}", sleep_time);
                //println!("sleep time 2 {:?}", sleep_time2);
                //println!("sleep duration {:?}", sleep_duration);

            }
           // println!("{:?}", );




    }

}



//Functions to collect airfoil performance data:
//lift and drag coefficient data is given for a set of discrete attack angles, 
//so then linear interpolation is used to determine the coefficients for the 
//attack angle that falls between the discrete angles.


//Given the angle of attack and status of the flaps,
//return lift angle coefficient for camabred airfoil with 
//plain trailing-edge (+/- 15 degree inflation).
fn lift_coefficient(angle: f64, flaps: i32) -> f64
{


    let clf0 = vec![-0.54, -0.2, 0.2, 0.57, 0.92, 1.21, 1.43, 1.4, 1.0]; //why cant i just make a number negative with '-'?...weird
    let clfd = vec![0.0, 0.45, 0.85, 1.02, 1.39, 1.65, 1.75, 1.38, 1.17];
    let clfu = vec![-0.74, -0.4, 0.0, 0.27, 0.63, 0.92, 1.03, 1.1, 0.78];
    let a = vec![-8.0, -4.0, 0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0];

    let mut cl: f64 = 0.0;

    for i in 0..8  
    {
        if a[i] <= angle && a[i + 1] > angle
        {
            if flaps == 0 //flaps not deflected
            {
                cl = clf0[i] - (a[i] - angle) * (clf0[i] - clf0[i + 1]) / (a[i] - a[i + 1]);
                break;
            }
            else if flaps == -1 //flaps down
            {
                cl = clfd[i] - (a[i] - angle) * (clfd[i] - clfd[i + 1]) / (a[i] - a[i + 1]);
                break;
            }
            else if flaps == 1 //flaps up
            {
                cl = clfu[i] - (a[i] - angle) * ( clfu[i] - clfu[i + 1]) / (a[i] - a[i + 1]);
                break;
            }
        }

    }
   // println!("{}", cl);
    return cl;
}


//given angle of attack and flap status, 
//return drag coefficient for cambered airfoil with 
//plain trailing-edge flap (+/- 15 degree deflection).
fn drag_coefficient(angle: f64, flaps: i32) -> f64
{
    let cdf0 = vec![0.01, 0.0074, 0.004, 0.009, 0.013, 0.023, 0.05, 0.12, 0.21];
    let cdfd = vec![0.0065, 0.0043, 0.0055, 0.0153, 0.0221, 0.0391, 0.1, 0.195, 0.3];
    let cdfu = vec![0.005, 0.0043, 0.0055, 0.02601, 0.03757, 0.06647, 0.13, 0.1, 0.25];
    let a = vec![-8.0, -4.0, 0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0];

    let mut cd: f64 = 0.75; //0.5 in book but 0.75 in actual code

    for i in 0..8  
    {
        if a[i] <= angle && a[i + 1] > angle
        {
            if flaps == 0 //flaps not deflected
            {
                cd = cdf0[i] - (a[i] - angle) * (cdf0[i] - cdf0[i + 1]) / (a[i] - a[i + 1]);
                break;
            }
            else if flaps == -1 //flaps down
            {
                cd = cdfd[i] - (a[i] - angle) * (cdfd[i] - cdfd[i + 1]) / (a[i] - a[i + 1]);
                break;
            }
            else if flaps == 1 //flaps up
            {
                cd = cdfu[i] - (a[i] - angle) * ( cdfu[i] - cdfu[i + 1]) / (a[i] - a[i + 1]);
                break;
            }
        }

    }
  //  println!("{}", cd);
    return cd;
}


//Rudder lift and drag coefficients are similar to that of the wing 
//but the coefficients themselves are different and the tail rudder 
//does not include flaps.
//Given attack angle, return lift coefficient for a symmetric (no camber) 
//airfoil without flaps.
fn rudder_lift_coefficient(angle: f64) -> f64
{
    let clf0 = vec![0.16, 0.456, 0.736, 0.968, 1.144, 1.12, 0.8];
    let a = vec![0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0];

    let mut cl: f64 = 0.0;
    let aa: f64 = angle.abs();

    for i in 0..6  
    {
        if a[i] <= aa && a[i + 1] > aa
        {
            cl = clf0[i] - (a[i] - aa) * (clf0[i] - clf0[i + 1]) / (a[i] - a[i + 1]);

            if angle < 0.0
            {
                cl = -cl;
            }
            break;
        }
    }
   // println!("{}", cl);
    return cl;
}

//Given attack angle, return drag coefficient for a symmetric (no camber) 
//airfoil without flaps.
fn rudder_drag_coefficient(angle: f64) -> f64
{
    let cdf0 = vec![0.0032, 0.0072, 0.0104, 0.0184, 0.04, 0.096, 0.168];
    let a = vec![0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0];

    let mut cd: f64 = 0.75; //0.5 in book
    let aa: f64 = angle.abs();

    for i in 0..6  
    {
        if a[i] <= aa && a[i + 1] > aa
        {
            cd = cdf0[i] - (a[i] - aa) * (cdf0[i] - cdf0[i + 1]) / (a[i] - a[i + 1]);

            break;
        }
    }
   // println!("{}", cd);
    return cd;
}