# bin_images

Data: Little-endian `u16` file.

## Usage

### Taking mean

`bin_images mean <IMG_PATH>`

#### Optional arguments

- `--out <OUT_PATH>`
- `--shift_data <SHIFT_PATH>`

### Shift detection

`bin_images detect <IMG_PATH>`

#### Optional arguments

- `--method <METHOD>`  
  Methods:
  - self-correlation (`cor`)
  - feature matching (`feature`)
  - maximum (`max`)
  - minimum (`min`)
