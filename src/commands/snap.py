

import gdsfactory as gf
import re
import argparse


def extract_and_save(gdsfile, regex_pattern, output_file):
    # Read the GDS file
    component = gf.read.import_gds(gdsfile)

    # Compile the regular expression
    pattern = re.compile(regex_pattern)

    # Open the output file for writing
    with open(output_file, 'w') as file:
        # Iterate through the references in the component
        for ref in component.references:
            ref_component = ref.ref
            # Check if the name of the reference component matches the pattern
            if pattern.match(ref_component.name):
                # Extract the position, rotation, and mirror information
                position = ref.position
                rotation = ref.rotation
                mirror = ref.x_reflection

                # Write the information to the file
                file.write(
                    f"Name: {ref_component.name}, Position: {position}, Rotation: {rotation}, Mirror: {mirror}\n")



def main():
    parser = argparse.ArgumentParser(description="Extract component information from a GDS file.")
    parser.add_argument('gdsfile', type=str, help="Path to the GDS file.")
    # parser.add_argument('--regex', type=str, default=r'your_regex_here', help="Regular expression pattern to match component names.")
    # parser.add_argument('--output', type=str, default='output.txt', help="Output file to save the extracted information.")
    args = parser.parse_args()

    # Regular expression pattern to match component names
    regex_pattern = r'^B[(.*)]'

    # Output file to save the extracted information
    output_file = 'extracted.txt'

    # Extract and save the information
    extract_and_save(args.gdsfile, regex_pattern, output_file)

if __name__ == "__main__":
    main()
