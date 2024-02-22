var MyElement = class extends LitElement {
    constructor() {
      super(...arguments);
      this.isInitialized = true;
    }
    render() {
      return x`
        <div>
            <h1>My Element</h1>
        </div>
      `;
    }
  };

export {
   MyElement
};
