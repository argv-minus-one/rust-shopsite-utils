use shopsite_aa::de as aa;
use std::{
	fs::{File, OpenOptions},
	io::{self, BufRead, BufReader, Write},
	num::NonZeroU8,
	path::PathBuf,
	process::exit,
	rc::Rc
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
	about = "Converts a ShopSite `.aa` file to JSON."
)]
struct Opts {
	/// Pretty-print the output JSON.
	#[structopt(short, long)]
	pretty: bool,

	/// Indent size, in spaces, to use when pretty-printing [default: 4]
	#[structopt(short = "s", long, requires = "pretty", conflicts_with = "indent-tabs")]
	indent_spaces: Option<NonZeroU8>,

	/// Use tabs instead of spaces for indentation when pretty-printing.
	#[structopt(short = "t", long, requires = "pretty")]
	indent_tabs: bool,

	/// JSON file to write to, instead of standard output.
	#[structopt(short, long)]
	output: Option<PathBuf>,

	/// .aa file to read from, instead of standard input.
	#[structopt(name = "FILE")]
	input: Option<PathBuf>
}

fn main() {
	let opts: Opts = Opts::from_args();

	let stdin = io::stdin();
	let stdout = io::stdout();

	let input: Box<dyn BufRead> = {
		if let Some(ref input_file) = opts.input {
			let open_result = File::open(input_file);

			match open_result {
				Ok(fh) => Box::new(BufReader::new(fh)),
				Err(error) => {
					eprintln!("Error opening input file {}: {}", input_file.to_string_lossy(), error);
					exit(1)
				}
			}
		}
		else {
			Box::new(stdin.lock())
		}
	};

	let output: Box<dyn Write> = {
		if let Some(ref output_file) = opts.output {
			let open_result = OpenOptions::new()
				.create(true)
				.write(true)
				.truncate(true)
				.open(output_file);

			match open_result {
				Ok(fh) => Box::new(fh),
				Err(error) => {
					eprintln!("Error opening output file {}: {}", output_file.to_string_lossy(), error);
					exit(1)
				}
			}
		}
		else {
			Box::new(stdout.lock())
		}
	};

	let de = aa::Deserializer::new(input, opts.input.map(Rc::from));

	// `serde_json::ser::Formatter` can't be used as a trait object, so we get to do this instead…
	fn do_transcode(mut de: aa::Deserializer<impl BufRead>, mut writer: impl Write, formatter: impl serde_json::ser::Formatter) -> Result<(), std::io::Error> {
		let mut ser = serde_json::Serializer::with_formatter(&mut writer, formatter);

		serde_transcode::transcode(&mut de, &mut ser)?;
		writeln!(&mut writer)?;
		writer.flush()
	}

	let result = {
		if opts.pretty {
			let mut indent_string_buf = Vec::<u8>::new();

			let indent_string: &[u8] = {
				if opts.indent_tabs {
					b"\t"
				}
				else if let Some(indent_spaces) = opts.indent_spaces {
					indent_string_buf.reserve_exact(indent_spaces.get() as usize);
					for _ in 0..indent_spaces.get() {
						indent_string_buf.push(b' ');
					}
					&indent_string_buf[..]
				}
				else {
					b"    "
				}
			};

			do_transcode(de, output, serde_json::ser::PrettyFormatter::with_indent(indent_string))
		}
		else {
			do_transcode(de, output, serde_json::ser::CompactFormatter)
		}
	};

	if let Err(error) = result {
		eprintln!("Error converting to JSON: {}", error);
		exit(1);
	}
}
