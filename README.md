# OpenAnim (CLI Edition)

OpenAnim automates the creation and execution of mathematical animations using Manim and OpenRouter. It generates Python code for Manim based on natural language descriptions and renders it immediately.

## Features

- **AI-Powered Code Generation**: Uses OpenRouter (openrouter/free) to generate Manim code from your descriptions.
- **Instant Rendering**: Automatically compiles and renders the generated animation.
- **Stream Output**: Watch the AI code generation process in real-time.
- **CLI Only**: Simple, lightweight command-line interface.

## Prerequisites

- **Python 3.10+** (Recommended)
- **Manim Community Edition** (`pip install manim`)
- **OpenAI Python Client** (`pip install openai`)
- **FFmpeg** (Required for Manim rendering)
- **OpenRouter API Key** (Get one at [openrouter.ai](https://openrouter.ai/))

## Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/OpenAnim.git
   cd OpenAnim
   ```

2. Install dependencies:

   ```bash
   pip install manim openai python-dotenv
   ```

   _Note: Manim has system dependencies (like FFmpeg). See [Manim Installation Guide](https://docs.manim.community/en/stable/installation.html)._

3. Set up your OpenRouter API key:
   Create a `.env` file in the project root:
   ```env
   OPENROUTER_API_KEY=your_api_key_here
   ```

## Usage

Run the tool with a description of the animation you want:

```bash
python app.py "Create a circle that transforms into a square"
```

Or run interactively:

```bash
python app.py
```

Then enter your prompt when asked.

The tool will:

1. Generate the Manim code (saved to `generated_scene.py`).
2. Ask for confirmation to render.
3. Render the animation (saved to `media/videos/generated_scene/ql/GenScene.mp4`).

## Example Prompts

- "Visualize the Pythagorean theorem."
- "Show a 3D rotating cube with text labels."
- "Animate a sine wave transforming into a cosine wave."

## Project Structure

- `app.py`: Main CLI application.
- `generated_scene.py`: Temporary file for generated Manim code.
- `.env`: API configuration.

## License

MIT License
