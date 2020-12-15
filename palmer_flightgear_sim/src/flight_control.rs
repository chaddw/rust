//This file contains the FlightControl System

//Specs
use specs::prelude::*;

//Flight control
use device_query::{DeviceQuery, DeviceState, Keycode};

//Exit program
use std::process;

//Get data needed for the System to work
use crate::data::KeyboardState;
use crate::data::DataFDM; //this actually isnt needed for the system to work... having an issue only using the one keyboardstate writestorage component

//System to handle user input
pub struct FlightControl;
impl<'a> System<'a> for FlightControl
{
    type SystemData = ( 
        ReadStorage<'a, DataFDM>, //we dont need fdm data for system to work
        WriteStorage<'a, KeyboardState>,
    );

    fn run(&mut self, (datafdm, mut keyboardstate): Self::SystemData) 
    {
        for (_fdm, keystate) in (&datafdm, &mut keyboardstate).join() 
        {
            //Set all states false before we know if they are being activated
            keystate.thrust_up = false; 
            keystate.thrust_down = false;
            keystate.aoa_up = false;
            keystate.aoa_down = false;
            keystate.bank_left = false;
            keystate.bank_right = false;
            keystate.flaps_down = false;
            keystate.zero_flaps = false;

            //Setup device query states
            let device_state = DeviceState::new();
            let keys: Vec<Keycode> = device_state.get_keys();

            //Thrust
            if keys.contains(&Keycode::A)
            {
                keystate.thrust_up = true;
            }
            else if keys.contains(&Keycode::Z)
            {
                keystate.thrust_down = true;
            }

            //angle of attack
            if keys.contains(&Keycode::Down)
            {
                keystate.aoa_up = true;
            }
            else if keys.contains(&Keycode::Up)
            {
                keystate.aoa_down = true;
            }

            //bank 
            if keys.contains(&Keycode::Left)
            {
                keystate.bank_left = true;
            }
            else if keys.contains(&Keycode::Right)
            {
                keystate.bank_right = true;
            }

            //flaps
            if keys.contains(&Keycode::F)
            {
                keystate.flaps_down = true;
            }
            else if keys.contains(&Keycode::G)
            {
                keystate.zero_flaps = true;
            }

            //Quit program
            if keys.contains(&Keycode::Q)
            {
                process::exit(1);
            }

        }//end for
    }//end run
}//end system