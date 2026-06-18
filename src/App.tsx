import "./App.css";
import { RegistryContext } from "./context/RegistryContext";
import { DashboardLayout } from "./components/dashboard/DashboardLayout";
import { useRegistry } from "./hooks/useRegistry";

function App() {
  const registry = useRegistry();

  return (
    <div data-testid="app-root" className="app-root">
      <RegistryContext.Provider value={registry}>
        <DashboardLayout />
      </RegistryContext.Provider>
    </div>
  );
}

export default App;
