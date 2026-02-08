package com.poziomki.app.ui.screen.event

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import com.poziomki.app.ui.theme.PoziomkiTheme
import org.koin.compose.viewmodel.koinViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventCreateScreen(
    onBack: () -> Unit,
    onCreated: () -> Unit,
) {
    val viewModel = koinViewModel<EventCreateViewModel>()
    val state by viewModel.state.collectAsState()

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Create Event") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier =
                Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .padding(PoziomkiTheme.spacing.lg),
        ) {
            OutlinedTextField(
                value = state.title,
                onValueChange = viewModel::updateTitle,
                label = { Text("Title") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            OutlinedTextField(
                value = state.description,
                onValueChange = viewModel::updateDescription,
                label = { Text("Description") },
                modifier = Modifier.fillMaxWidth(),
                maxLines = 4,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            OutlinedTextField(
                value = state.location,
                onValueChange = viewModel::updateLocation,
                label = { Text("Location") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            OutlinedTextField(
                value = state.startsAt,
                onValueChange = viewModel::updateStartsAt,
                label = { Text("Start date (YYYY-MM-DD HH:mm)") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            state.error?.let {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                Text(text = it, color = androidx.compose.material3.MaterialTheme.colorScheme.error)
            }

            Spacer(modifier = Modifier.weight(1f))

            Button(
                onClick = { viewModel.createEvent(onCreated) },
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(PoziomkiTheme.componentSizes.buttonHeight),
                enabled = !state.isLoading && state.title.isNotBlank() && state.startsAt.isNotBlank(),
            ) {
                if (state.isLoading) CircularProgressIndicator() else Text("Create Event")
            }
        }
    }
}
