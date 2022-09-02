use std::collections::HashMap;
use std::time::SystemTime;
use bson::Bson;

// the data manager currently holds all data received from the data plugins
// it will in future manage the data in a way, that it's, if required, loaded
// from a time series database
// but at the moment the size of the data is very small, so a not irrelevant
// amount of data can easily be stored in RAM
// for every data plugin it stores at max the last 10_000 values

pub struct DataManager {
    values: HashMap<String, Vec<(SystemTime, Bson)>>,
}

const MAX_VALUES: usize = 10000;

impl DataManager {
    pub fn new() -> Self {
        DataManager {
            values: HashMap::new(),
        }
    }


    pub fn add(&mut self, name: String, time: SystemTime, value: Bson) {
        // get corresponding data vector
        let vec = self.values.entry(name).or_insert(vec![]);

        // add data
        vec.push((time, value));

        // if more the MAX_VALUES, remove the oldest
        if vec.len() > MAX_VALUES {
            vec.remove(0);
        }
    }

    pub fn get_last(&self, name: String, x: usize) -> Option<Vec<(SystemTime, Bson)>> {
        // is there SOMETHING stored in the values hash map?
        let value_name = self.values.get(&name);
        if value_name.is_none() { return None; }

        // ..yes, so copy the last x values and return them
        let mut collected = vec![];
        for datum in value_name.unwrap().iter().rev() {
            if collected.len() >= x {
                break;
            }
            collected.push((datum.0, datum.1.clone()));
        }
        return Some(collected);
    }
}
