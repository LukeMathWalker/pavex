from markdown.preprocessors import Preprocessor
from markdown.extensions import Extension

class TrailingSlashPreprocessor(Preprocessor):
    def run(self, lines):
        new_lines = []
        for line in lines:
            # If the line ends with a trailing backslash, replace it with an HTML <br>
            if line.endswith('\\'):
                # Remove the slash and add the <br> tag.
                new_line = line[:-1] + "<br>"
                new_lines.append(new_line)
            else:
                new_lines.append(line)
        return new_lines

class TrailingSlashExtension(Extension):
    def extendMarkdown(self, md):
        # Register our preprocessor with a priority that ensures it runs before standard processing.
        md.preprocessors.register(TrailingSlashPreprocessor(md), 'trailing_slash_preprocessor', 25)

def makeExtension(**kwargs):
    return TrailingSlashExtension(**kwargs)
