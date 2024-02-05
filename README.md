# bin_images

Data: Little-endian `u16` file.

## Usage

### Taking mean

`bin_images mean <PATH>`

#### Optional arguments

- `--out <OUT_PATH>`
- `--shift_data <SHIFT_DATA_PATH>`

### Drift detection

`bin_images detect <PATH>`

#### Optional arguments

- `--method <METHOD>`  
  Methods:
  - self-correlation (`cor`)
  - feature matching (`feature`)
  - maximum (`max`)
  - minimum (`min`)
