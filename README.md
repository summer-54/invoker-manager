# invoker-manager
Server tool which keeps connections to invokers and distribute submissions between them.

## Run in container
```bash
podman build -t localhost/invoker-manager .
podman run -d -e INVOKERS_ADDRESS=0.0.0.0:1111 -e TS_ADDRESS=0.0.0.0:2222 -e CP_ADDRESS=0.0.0.0:3333 -p 1111:1111 -p 2222:2222 -p 3333:3333 docker.io/a1exeyy/invoker-manager
```

## invoker-manager → testing-system
### Submission verdict
```
TYPE VERDICT
SUBMISSION <submission-uuid>
VERDICT <verdict>
// if <verdict> == OK {
SUM <sum>
GROUPS <points for 0 group> ... <points for n group>
// } else {
MESSAGE <error message>
// }
DATA
<verdict> <time> <memory> // 1 test result
...
<verdict> <time> <memory> // n test result
```
#### Examples:
```
TYPE VERDICT
SUBMISSION cc67b6ff-471b-b262-b6de-42d4c8e2fab1
VERDICT CE
MESSAGE Rust isn't C++
DATA
```
```
TYPE VERDICT
SUBMISSION cc67b6ff-471b-b262-b6de-42d4c8e2fab1
VERDICT OK
SUM 90
GROUPS 50 20 20 0
DATA
OK 0.1 54
OK 0.4 54
OK 0.9 54
TL 1.0 54
```

### Test verdict
```
TYPE TEST
SUBMISSION <submission-uuid>
TEST <test number>
VERDICT <verdict>
DATA
<data>
```
## invoker-manager ← testing-system
```
<uuid [16 bites]><test count [2 bites]><data>
```
## invoker-manager ←→ invoker

## Verdicts

Standart verdicts and new one -- SK = Skipped
You can find it in [invoker repository](https://github.com/summer-54/invoker).

## Control-panel

### GET -> /control-panel/invokers-status
Gets list of invokers and which submission they are testing.

```bash
$ curl ${CP_ADDRESS}/control-panel/invokers-status
```

```
{
    ...
    <invoker uuid [Uuid]> : <[None]> | <submission uuid [Uuid]> 
    ...
}
```

### GET -> /control-panel/tests-results
Gets tests results of currently testing submissions

```bash
$ curl ${CP_ADDRESS}/control-panel/tests-results
```

```
{
    ...
    <submission uuid [Uuid]> : [
        ...
        {
            verdict: <test verdict [string | Verdict]>,
            time: <used time on test [f32 | seconds]>,
            memory: <used memory on test [u32 | bytes]>,
        },
        ...
    ]
    ...
}
```

### DELETE /control-panel/invokers/{invoker uuid} (IN PROGRESS)
Dedlete invoker by invoker uuid

```bash
$ curl ${CP_ADDRESS}/control-panel/invokers/{uuid | xxxxxxxx-xxxx-Mxxx-Nxxx-xxxxxxxxxxxx} -X DELETE
```

