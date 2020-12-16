//This file contains all of the data required by the Systems
//The Components are: KeyboardState, EquationsOfMotion, Packet 
//The structures that are part of some Components are: PointMass, FGNetFDM
//The resources are: DeltaTime, MaxThrust, DeltaThrust

//SPECS
use specs::prelude::*;

//Converting FGNetFDM struct to bytes to be sent as a packet
use serde::{Deserialize, Serialize};

//Vector, Matrix, Quaternion module
use crate::common::Myvec;
use crate::common::Mymatrix;
use crate::common::Myquaternion;

//Time step (delta time) shared resource
#[derive(Default)]
pub struct DeltaTime(pub f32);

//Max thrust potential resource
#[derive(Default)]
pub struct MaxThrust(pub f32);

//Delta thrust increment
#[derive(Default)]
pub struct DeltaThrust(pub f32);

//Component state machine for keyboard presses
#[derive(Debug)]
pub struct KeyboardState
{
    pub thrust_up: bool,
    pub thrust_down: bool,
    pub left_rudder: bool,
    pub right_rudder: bool,
    pub roll_left: bool,
    pub roll_right: bool,
    pub pitch_up: bool,
    pub pitch_down: bool,
    pub flaps_down: bool,
    pub zero_flaps: bool,
}
impl Component for KeyboardState
{
    type Storage = VecStorage<Self>;
}





//Elements making up the bodystructure, this is part of the DataFDM component
#[derive(Debug)]
pub struct PointMass
{
    pub f_mass: f32,
    pub v_d_coords: Myvec, //"design position"
    pub v_local_inertia: Myvec,
    pub f_incidence: f32,
    pub f_dihedral: f32,
    pub f_area: f32,
    pub i_flap: i32,
    pub v_normal: Myvec,
    pub v_cg_coords: Myvec //"corrected position"
}

//Component containing data on the airplane
#[derive(Debug, Default)]
pub struct DataFDM
{
    pub mass: f32, //total mass
    pub m_inertia: Mymatrix,
    pub m_inertia_inverse: Mymatrix,
    pub v_position: Myvec, // position in earth coordinates
    pub v_velocity: Myvec, // velocity in earth coordinates
    pub v_velocity_body: Myvec, // velocity in body coordinates
    pub v_angular_velocity: Myvec, // angular velocity in body coordinates
    pub v_euler_angles: Myvec,   
    pub f_speed: f32, // speed (magnitude of the velocity)
    pub stalling: bool,
    pub flaps: bool,
    pub q_orientation: Myquaternion, // orientation in earth coordinates 
    pub v_forces: Myvec, // total force on body
    pub thrustforce: f32, // magnitude of thrust
    pub v_moments: Myvec, // total moment (torque) on body
    pub element: Vec<PointMass>, // vector of point mass elements
    pub current_frame: usize,
}
impl Component for DataFDM
{
    type Storage = VecStorage<Self>;
}




//Component containg the the FGNetFDM structure, and its conversion into bytes
#[derive(Debug, Default)]
pub struct Packet
{
    pub fgnetfdm: FGNetFDM,
    pub bytes: Vec<u8>,
}
impl Component for Packet
{
    type Storage = VecStorage<Self>;
}

//Component for making a network packet to be sent to FlightGear
#[derive(Debug, Default, Serialize, Deserialize)]
#[repr(C)] 
pub struct FGNetFDM
{
    pub version: u32, // increment when data values change
    padding: f32, // padding

    // // Positions
    pub longitude: f64, // geodetic (radians)
    pub latitude: f64, // geodetic (radians)
    pub altitude: f64, // above sea level (meters)
    agl: f32, // above ground level (meters)
    pub phi: f32, // roll (radians)
    pub theta: f32, // pitch (radians)
    pub psi: f32, // yaw or true heading (radians)
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

