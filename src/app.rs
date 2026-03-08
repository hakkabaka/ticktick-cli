use crate::ticktick::{ApiClient, ProjectSummary, Task, TaskSummary};

#[derive(Debug)]
pub enum CurrentScreen {
    ProjectsView,
    ProjectView,
    TicketView,
}

#[derive(Debug)]
pub struct ProjectWithTasks {
    pub project: ProjectSummary,
    pub tasks: Vec<TaskSummary>,
}

#[derive(Debug)]
pub struct App {
    pub current_screen: CurrentScreen,
    pub should_exit: bool,
    pub selected_project: usize,
    pub selected_task: usize,
    pub projects: Vec<ProjectWithTasks>,
    pub viewed_ticket: Option<Task>,
    pub last_error: Option<String>,
}

impl App {
    pub fn new(projects: Vec<ProjectWithTasks>) -> Self {
        Self {
            current_screen: CurrentScreen::ProjectsView,
            should_exit: false,
            selected_project: 0,
            selected_task: 0,
            projects,
            viewed_ticket: None,
            last_error: None,
        }
    }

    pub fn next_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        self.selected_project = (self.selected_project + 1) % self.projects.len();
    }

    pub fn previous_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        self.selected_project = if self.selected_project == 0 {
            self.projects.len() - 1
        } else {
            self.selected_project - 1
        };
    }

    pub async fn open_project_tickets(&mut self, client: &ApiClient) {
        if self.projects.is_empty() {
            return;
        }

        let project_id = self.projects[self.selected_project].project.id.clone();
        match client.get_project_data(&project_id).await {
            Ok(data) => {
                self.projects[self.selected_project].tasks = data.tasks;
                self.last_error = None;
            }
            Err(err) => {
                self.last_error = Some(format!("Failed to load project tickets: {err}"));
            }
        }

        self.selected_task = 0;
        self.viewed_ticket = None;
        self.current_screen = CurrentScreen::ProjectView;
    }

    pub fn back(&mut self) {
        match self.current_screen {
            CurrentScreen::ProjectsView => {}
            CurrentScreen::ProjectView => self.current_screen = CurrentScreen::ProjectsView,
            CurrentScreen::TicketView => {
                self.current_screen = CurrentScreen::ProjectView;
                self.viewed_ticket = None;
            }
        }
        self.last_error = None;
    }

    pub fn next_task(&mut self) {
        if let Some(project) = self.projects.get(self.selected_project) {
            if project.tasks.is_empty() {
                return;
            }
            self.selected_task = (self.selected_task + 1) % project.tasks.len();
        }
    }

    pub fn previous_task(&mut self) {
        if let Some(project) = self.projects.get(self.selected_project) {
            if project.tasks.is_empty() {
                return;
            }
            self.selected_task = if self.selected_task == 0 {
                project.tasks.len() - 1
            } else {
                self.selected_task - 1
            };
        }
    }

    pub fn selected_project(&self) -> Option<&ProjectWithTasks> {
        self.projects.get(self.selected_project)
    }

    pub async fn open_ticket_details(&mut self, client: &ApiClient) {
        let Some(project) = self.projects.get(self.selected_project) else {
            return;
        };
        let project_id = project.project.id.clone();
        let Some(task) = project.tasks.get(self.selected_task) else {
            return;
        };
        let task_id = task.id.clone();

        match client.get_task_by_id(&project_id, &task_id).await {
            Ok(detail) => {
                self.viewed_ticket = Some(detail);
                self.last_error = None;
            }
            Err(err) => {
                self.viewed_ticket = None;
                self.last_error = Some(format!("Failed to load ticket details: {err}"));
            }
        }
        self.current_screen = CurrentScreen::TicketView;
    }

    pub fn quit(&mut self) {
        self.should_exit = true;
    }
}
