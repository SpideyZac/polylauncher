'''
Script to clean HAR files by removing unnecessary fields from each entry.
'''
import json


def clean_har(input_file):
    '''
    Only keeps the URL from each entry in the HAR file.
    '''
    with open(input_file, 'r', encoding='utf-8') as f:
        har_data = json.load(f)

    cleaned_entries = []
    for entry in har_data.get('log', {}).get('entries', []):
        if entry['request']['method'] != 'GET':
            continue

        if entry['request']['method'].split('/')[-1] == 'favicon.ico':
            continue

        cleaned_entries.append(entry['request']['url'])

    with open(input_file, 'w', encoding='utf-8') as f:
        json.dump(cleaned_entries, f)


if __name__ == '__main__':
    import sys
    if len(sys.argv) != 2:
        print("Usage: python clean_har.py <input_har_file>")
        sys.exit(1)

    input_har_file = sys.argv[1]
    clean_har(input_har_file)
