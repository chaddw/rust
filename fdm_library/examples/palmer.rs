//This main file sets up the ECS architecture, creates the entity, and runs the main simulation loop

//To run from the commmand line: cargo.exe run --example palmer

//FlightGear is ran with this line of command argumments on the fgfs executable:
//fgfs.exe --aircraft=ufo --disable-panel --disable-sound --enable-hud --disable-random-objects --fdm=null --timeofday=noon --native-fdm=socket,in,30,,5500,udp

//Cessna Skyhawk visual
//fgfs.exe --disable-panel --disable-sound --enable-hud --disable-random-objects --fdm=null --timeofday=noon --native-fdm=socket,in,30,,5500,udp

//SPECS
use specs::prelude::*;

//Coordinate transforms/nalgebra vector
use coord_transforms::prelude::*;

//Main loop
use std::{thread, time};

//Import Component modules
use fdm_library::palmer::fdm::structures::KeyboardState;
use fdm_library::palmer::fdm::structures::DataFDM;
use fdm_library::palmer::fdm::structures::PerformanceData;
use fdm_library::flightgear::FGNetFDM;

//Import Resources
use fdm_library::palmer::resources::delta_time::DeltaTime;

//Import Systems
use fdm_library::palmer::systems::system_flight_control::FlightControl;
use fdm_library::palmer::systems::system_equations_of_motion::EquationsOfMotion;
use fdm_library::palmer::systems::system_make_packet::MakePacket;
use fdm_library::palmer::systems::system_send_packet::SendPacket;


fn main()
{
    //Create world
    let mut world = World::new();
    
    //Register Components in the world
    world.register::<DataFDM>();
    world.register::<KeyboardState>();
    world.register::<FGNetFDM>();

    //Choose frame rate, which will calculate delta time
    let frame_rate: f64 = 30.0;
    let dt: f64 = 1.0 / frame_rate; //seconds

    //Add dt as a SPECS resource
    world.insert(DeltaTime(dt)); 

    //Create variable to keep track of time elapsed
    let mut current_time = 0.0;
    let mut current_frame_main: usize = 0;
   
    //Create dispatcher of the Systems
    let mut dispatcher = DispatcherBuilder::new()
    .with(FlightControl, "flightcontrol", &[])
    .with(EquationsOfMotion, "EOM", &["flightcontrol"])
    .with(MakePacket, "makepacket", &["EOM"])
    .with(SendPacket, "sendpacket", &["makepacket"])
    .build();
    dispatcher.setup(&mut world);

    //Create airplane Entity with Components
    let _plane = world.create_entity()
    .with(DataFDM{
        //Starting position and origin in geodetic coordinates
        //Wpafb runway latitude/longitude/altitude. Ground level is 248.0 meters elevation
        //Note: make the origin and start position the same, origin will remain constant throughout simulation
        position: Vector3::new(39.826, -84.045, 248.0),
        lla_origin: Vector3::new(39.826, -84.045, 248.0),
       
        bank: 0.0, //bank angle degrees (-20 - 20)
        alpha: 0.0,//angle of attack degrees (-16 - 20)
        throttle: 0.0, //throttle percentage (0 - 1)
        flap: 0.0,  //flap deflection amount degrees (20 or 40)
        airspeed: 0.0,

        climb_angle: 0.0,
        heading_angle: 0.0,
        climb_rate: 0.0,
        q: vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0], //will store ODE results
        
        mass_properties: PerformanceData{
            wing_area: 16.2,            //  wing wetted area, m^2
            wing_span: 10.9,            //  wing span, m
            tail_area: 2.0,             //  tail wetted area, m^2
            cl_slope0: 0.0889,          //  slope of Cl-alpha curve
            cl0: 0.178,                 //  Cl value when alpha = 0
            cl_slope1: -0.1,            //  slope of post-stall Cl-alpha curve
            cl1: 3.2,                   //  intercept of post-stall Cl-alpha curve
            alpha_cl_max: 16.0,         //  alpha at Cl(max)
            cdp: 0.034,                 //  parasitic drag coefficient
            eff: 0.77,                  //  induced drag efficiency coefficient
            mass: 1114.0,               //  airplane mass, kg
            engine_power: 119310.0,     //  peak engine power, W
            engine_rps: 40.0,           //  engine turnover rate, rev/s
            prop_diameter: 1.905,       //  propeller diameter, m
            a: 1.83,                    //  propeller efficiency curve fit coefficient
            b:-1.32,                    //  propeller efficiency curve fit coefficient
        }

    })
    .with(KeyboardState{
        throttle_up: false,
        throttle_down: false,
        aoa_up: false,
        aoa_down: false,
        bank_right: false,
        bank_left: false,
        flaps_down: false,
        zero_flaps: false,
    })
    .with(FGNetFDM{
        ..Default::default()
    })
    .build();

    //Create time type with the dt in milliseconds
    let timestep = time::Duration::from_millis((dt * 1000.0) as u64);

    //Loop simulation
    loop 
    {
        //Get current time
        let start = time::Instant::now();

        //Increment time count
        current_time = current_time + dt;
        current_frame_main = current_frame_main + 1;
        println!("{}", "====================================================");
        println!("Time (seconds): {}, Frames: {}", current_time, current_frame_main);

        //Process this frame
        dispatcher.dispatch(&world);
        world.maintain();

        //Find difference in time elapsed this loop versus the timestep
        let sleep_time = timestep.checked_sub(time::Instant::now().duration_since(start));

        //Sleep for extra time if calculation took less time than the DT time step
        if sleep_time != None 
        {
            thread::sleep(sleep_time.unwrap());
        }
    }
}