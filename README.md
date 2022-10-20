

# task_local
```rust
use osiris::{runtime_local, task::task_id}; 

task_local! {
    static task_id: usize = task_id(); 
}; 
```

# spawn

```rust

use osiris::spawn; 


let task = spawn(async {

}); 