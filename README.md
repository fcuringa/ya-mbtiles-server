# Yet Another MBTiles server

MBTiles server with custom, cacheable authentication support from Python script.

Uses the filesystem as storage backend, support for actual MBTiles files is planned in the future.

## Installation

See the releases page.

## Usage

```
Serves MBTiles really fast. Supports custom authentication and using filesystem as storage backend.

Usage: ya-mbtiles-server [OPTIONS]

Options:
      --port [<PORT>]                 Port to listen on [default: 3000]
      --route [<ROUTE>]               Route prefix for serving MBTiles [default: /mbtiles]
      --webroot [<WEBROOT>]           Webroot for serving tiles [default: ./example-data]
      --tilesmode [<TILES_MODE>]      The serving mode for MBTiles, either 'filesystem' or 'mbtiles'. The former assumes the tile data is stored directly as individual files, the latter assumes a flat directory of MBTiles files. [default: mbtiles]
      --authscript [<AUTH_SCRIPT>]    Python script for authentication [default: auth.py]
      --authheaders [<AUTH_HEADERS>]  Request headers with authorization data, if you need several of them use commas to separate [default: ]
      --cachetime [<CACHE_TIME>]      Cache validity in seconds [default: 3600]
  -h, --help                          Print help
  -V, --version                       Print version
```

In more details:

- *port*: The server port.
- *route*: The prefix at which the tiles will be served from.
- *webroot*: the path to the directory where the files to be served are located.
- *tilesmode*: the storage backend for serving the files, see below.
- *authscript*: the python script to be used for authentication. Refer to the format below.
- *authheaders*: the headers used for authentication, those will be passed to your script. They will also be used 
  for caching you script responses. The value of each header will be used to create a hash so your script is only called
  when required.
- *cachetime*: the validity of the cache entries created from your script responses.

When `tilesmode=filesystem`, the webroot directory structure is assumed to be:

```
<WEBROOT>
└── <MAP_NAME_1>
    └── <ZOOM_LEVEL>
        └── <TILE_ROW_1>
            └── <TILE_COLUMN_1>
            └── <TILE_COLUMN_2>
        └── <TILE_ROW_2>
            └── <TILE_COLUMN_1>
            └── <TILE_COLUMN_2>
└── <MAP_NAME_2>
```

When `tilesmode=mbtiles`, the webroot directory structure is assumed to be:

```
<WEBROOT>
├── <MAP_NAME_1>.mbtiles
└── <MAP_NAME_2>.mbtiles
```

### Python authentication script

Provide a script with this format:

```python
def auth(path, **headers):
    """
    Authentication middleware.
    :param path: The request path
    :param headers: The request headers, only the ones requested in REQ_HEADERS parameter are included
    :return: True if the authentication is successful, False otherwise
    """
    print(path, headers)
    return True
```

If you wish to use a virtual environment, make sure `ya-mbtiles-server` is run with this environment activated.

## Test the server

An example using this server is provided at `index.html`. 

Make sure the server is running locally on the port 3000 and open the page using your preferred web browser.

The server will - by default - serve the MBtiles files under `example-data/`, those were obtained from: 
[klokantech/mapbox-gl-js-offline-example](https://github.com/klokantech/mapbox-gl-js-offline-example).