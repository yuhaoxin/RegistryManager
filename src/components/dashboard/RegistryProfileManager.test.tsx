import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { RegistryProfileManager } from "./RegistryProfileManager";

const mockProfiles = [
  { id: "p1", name: "Local", registryUrl: "http://localhost:5000", credentialRef: null, createdAt: "", updatedAt: "" },
  { id: "p2", name: "Staging", registryUrl: "http://localhost:5001", credentialRef: "cred-1", createdAt: "", updatedAt: "" },
];

function renderManager(props?: Partial<React.ComponentProps<typeof RegistryProfileManager>>) {
  return render(
    <RegistryProfileManager
      profiles={mockProfiles}
      onSelect={vi.fn()}
      onCreate={vi.fn().mockResolvedValue(undefined)}
      onUpdate={vi.fn().mockResolvedValue(undefined)}
      onDelete={vi.fn().mockResolvedValue(undefined)}
      {...props}
    />,
  );
}

describe("RegistryProfileManager", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("renders profile list", () => {
    renderManager();
    const items = screen.getAllByTestId("rm-profile-item");
    expect(items).toHaveLength(2);
    expect(screen.getByText("Local")).toBeInTheDocument();
    expect(screen.getByText("Staging")).toBeInTheDocument();
  });

  it("keeps action buttons outside profile option content", () => {
    renderManager();

    expect(screen.queryByRole("listbox", { name: "Registry 配置" })).not.toBeInTheDocument();
    expect(screen.queryAllByRole("option")).toHaveLength(0);
    expect(screen.getAllByTestId("rm-profile-edit-button")).toHaveLength(2);
    expect(screen.getAllByTestId("rm-profile-delete-button")).toHaveLength(2);
  });

  it("shows empty message when no profiles exist", () => {
    renderManager({ profiles: [] });
    expect(screen.getByTestId("rm-no-profiles-message")).toBeInTheDocument();
  });

  it("calls onSelect when a profile is selected", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    renderManager({ onSelect });

    const radios = screen.getAllByTestId("rm-profile-radio");
    await user.click(radios[1]);

    expect(onSelect).toHaveBeenCalledWith(mockProfiles[1]);
  });

  it("creates a new profile", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);
    renderManager({ profiles: [], onCreate });

    await user.click(screen.getByTestId("rm-add-profile-button"));
    await user.type(screen.getByTestId("rm-profile-name-input"), "Production");
    await user.type(screen.getByTestId("rm-profile-url-input"), "http://registry.example.com");
    await user.click(screen.getByTestId("rm-profile-save-button"));

    expect(onCreate).toHaveBeenCalledWith({
      name: "Production",
      registryUrl: "http://registry.example.com",
      credentialRef: null,
    });
  });

  it("updates an existing profile", async () => {
    const user = userEvent.setup();
    const onUpdate = vi.fn().mockResolvedValue(undefined);
    renderManager({ onUpdate });

    const editButtons = screen.getAllByTestId("rm-profile-edit-button");
    await user.click(editButtons[0]);
    await user.clear(screen.getByTestId("rm-profile-name-input"));
    await user.type(screen.getByTestId("rm-profile-name-input"), "Updated");
    await user.click(screen.getByTestId("rm-profile-save-button"));

    expect(onUpdate).toHaveBeenCalledWith("p1", {
      name: "Updated",
      registryUrl: "http://localhost:5000",
      credentialRef: null,
    });
  });

  it("displays duplicate URL error when creating a profile", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockRejectedValue({
      code: "duplicate_registry_url",
      message: "已存在使用此 URL 的 Registry 配置。",
    });
    renderManager({ profiles: [], onCreate });

    await user.click(screen.getByTestId("rm-add-profile-button"));
    await user.type(screen.getByTestId("rm-profile-name-input"), "Duplicate");
    await user.type(screen.getByTestId("rm-profile-url-input"), "http://localhost:5000");
    await user.click(screen.getByTestId("rm-profile-save-button"));

    const error = await screen.findByTestId("rm-profile-error");
    expect(error).toHaveTextContent("已存在使用此 URL 的 Registry 配置。");
    expect(screen.getByTestId("rm-profile-form")).toBeInTheDocument();
  });

  it("displays duplicate URL error when editing a profile", async () => {
    const user = userEvent.setup();
    const onUpdate = vi.fn().mockRejectedValue({
      code: "duplicate_registry_url",
      message: "已存在使用此 URL 的 Registry 配置。",
    });
    renderManager({ onUpdate });

    const editButtons = screen.getAllByTestId("rm-profile-edit-button");
    await user.click(editButtons[0]);
    await user.clear(screen.getByTestId("rm-profile-url-input"));
    await user.type(screen.getByTestId("rm-profile-url-input"), "http://localhost:5001");
    await user.click(screen.getByTestId("rm-profile-save-button"));

    const error = await screen.findByTestId("rm-profile-error");
    expect(error).toHaveTextContent("已存在使用此 URL 的 Registry 配置。");
    expect(screen.getByTestId("rm-profile-form")).toBeInTheDocument();
  });

  it("deletes a profile after inline confirmation", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn().mockResolvedValue(undefined);
    renderManager({ onDelete });

    const deleteButtons = screen.getAllByTestId("rm-profile-delete-button");
    await user.click(deleteButtons[0]);
    await user.click(screen.getByTestId("rm-profile-delete-confirm-button"));

    expect(onDelete).toHaveBeenCalledWith("p1");
  });

  it("displays an error when deleting a profile fails", async () => {
    const user = userEvent.setup();
    const onDelete = vi.fn().mockRejectedValue({ message: "删除失败" });
    renderManager({ onDelete });

    const deleteButtons = screen.getAllByTestId("rm-profile-delete-button");
    await user.click(deleteButtons[0]);
    await user.click(screen.getByTestId("rm-profile-delete-confirm-button"));

    const error = await screen.findByTestId("rm-profile-delete-error");
    expect(error).toHaveTextContent("删除失败");
  });
});
