# al-crunch-pool

`al-crunch-pool` executes jobs on multiple threads.

It is especially suited for number-chrunching algorithms that can be
efficiently parallelized but where the jobs are dynamically generated.

- jobs are specified as closures
  - they are distributed among different worker threads through a bounded queue
  - they can spawn new jobs
  - they are executed synchronously when the queue is full
- worker are created via `sys::thread::spawn`
  - each thread gets its own worker-state that is made available to the jobs it executes
  - the threads are gracefully shutdown when the pool joins


## Performance

Scenario
- inspect 1M directories in a 3-level balanced tree
- output to /dev/null
- hot caches

| What                 | Version | Time  | Energy |
| -------------------- | ------- | ----- | ------ |
| GNU findutils        | 4.9.0   | 6.4s  |   100J |
| fdfind               | 8.6.0   | 1.37s |    49J |
| jwalk		       | 0.8.1   | 1.23s |    44J |
| dust		       | 0.8.5   | 0.98s |    30J |
| pdu		       | 0.9.0   | 0.60s |    21J |
| du - crunch-pool     | 0.1.0   | 0.58s |    21J | 
| find - crunch-pool   | 0.1.0   | 0.52s |    19J | 
| du-libc - crunch-pool| 0.1.0   |*0.46s*|  *17J* | 
