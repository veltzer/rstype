# Content Sources

`rstype` supports multiple sources for typing practice. You can configure the
default source in your config file or override it with the `--source` flag
when running the `train` command.

---

## Wikipedia

The Wikipedia source fetches random, high-quality paragraphs from a local
collection. This provides varied, natural language practice on a wide range
of topics.

### Commands

*   **`rstype wikipedia download`**
    Downloads paragraphs from Wikipedia until your local collection reaches
    the target size (default: 1000). Use `-c` or `--count` to specify a
    different total count.
    ```bash
    rstype wikipedia download --count 5000
    ```

*   **`rstype wikipedia stats`**
    Displays statistics about your local collection, including total
    paragraphs and file size.

*   **`rstype wikipedia clear`**
    Deletes your local collection.

*   **`rstype wikipedia show`**
    Shows the file path where the Wikipedia collection is stored.

---

## Word Salad (Dictionaries)

Word salad mode generates practice text by picking random words from
installed dictionaries. This is excellent for drilling common words and
improving raw speed without the context of natural sentences.

### Commands

*   **`rstype dict list-remote`**
    Lists all language dictionaries available for installation from
    the [wooorm/dictionaries](https://github.com/wooorm/dictionaries) collection.

*   **`rstype dict install <LANG>`**
    Installs a specific dictionary (e.g., `en-US`, `de-DE`, `fr`).
    ```bash
    rstype dict install en-US
    ```

*   **`rstype dict list`**
    Lists all dictionaries currently installed on your system.

*   **`rstype dict remove <LANG>`**
    Removes an installed dictionary.

*   **`rstype dict show`**
    Shows the directory path where dictionaries are stored.

---

## Usage in Training

To use a specific source during a training session:

```bash
# Train with Wikipedia paragraphs
rstype train --source wikipedia

# Train with Word Salad (uses the first available installed dictionary)
rstype train --source word-salad
```

You can also specify the target length of the text:

```bash
# Possible lengths: one-line, short-paragraph, paragraph, long-paragraph
rstype train --source wikipedia --length short-paragraph
```
