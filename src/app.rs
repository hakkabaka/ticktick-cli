use crate::ticktick::{ProjectSummary, TaskSummary};

#[derive(Debug)]
pub enum CurrentScreen {
    ProjectsView,
    ProjectView,
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
}

impl App {
    pub fn new(projects: Vec<ProjectWithTasks>) -> Self {
        Self {
            current_screen: CurrentScreen::ProjectsView,
            should_exit: false,
            selected_project: 0,
            selected_task: 0,
            projects,
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

    pub fn open_project_tickets(&mut self) {
        if self.projects.is_empty() {
            return;
        }
        self.selected_task = 0;
        self.current_screen = CurrentScreen::ProjectView;
    }

    pub fn back_to_projects(&mut self) {
        self.current_screen = CurrentScreen::ProjectsView;
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

    pub fn quit(&mut self) {
        self.should_exit = true;
    }
}
