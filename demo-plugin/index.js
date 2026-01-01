class DemoWidget extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._config = {};
    this._message = null;
  }

  set config(val) {
    this._config = val;
    this.render();
  }

  set message(val) {
    this._message = val;
    this.render();
  }

  connectedCallback() {
    this.render();
  }

  render() {
    const topic = this._config.topic || 'No topic';
    const payload = this._message ? this._message.payload : 'Waiting...';
    
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
          width: 100%;
          height: 100%;
          background: #f0f0f0;
          color: #000;
          padding: 10px;
          box-sizing: border-box;
          overflow: auto;
          border-radius: 4px;
        }
        @media (prefers-color-scheme: dark) {
            :host {
                background: #333;
                color: #fff;
            }
        }
        h2 { margin: 0; font-size: 1.2em; }
        .value { font-size: 2em; font-weight: bold; margin: 10px 0; }
        .meta { font-size: 0.8em; opacity: 0.7; }
        button {
            padding: 5px 10px;
            cursor: pointer;
        }
      </style>
      <div>
        <h2>Demo Widget</h2>
        <div class="meta">Topic: ${topic}</div>
        <div class="value">${payload}</div>
        <button id="btn">Send Hello</button>
      </div>
    `;

    const btn = this.shadowRoot.getElementById('btn');
    if (btn) {
        btn.onclick = () => {
          if (window.MqttViewerSDK && this._config.topic) {
            window.MqttViewerSDK.publish(this._config.topic, 'Hello from Plugin!');
          } else {
              alert('SDK not ready or topic not set');
          }
        };
    }
  }
}

customElements.define('demo-widget', DemoWidget);
