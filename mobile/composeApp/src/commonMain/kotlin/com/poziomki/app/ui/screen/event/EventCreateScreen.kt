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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import com.poziomki.app.api.ApiResult
import com.poziomki.app.api.ApiService
import com.poziomki.app.api.CreateEventRequest
import com.poziomki.app.ui.theme.PoziomkiTheme
import kotlinx.coroutines.launch
import org.koin.compose.koinInject

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun EventCreateScreen(
    onBack: () -> Unit,
    onCreated: () -> Unit,
) {
    val apiService = koinInject<ApiService>()
    val scope = rememberCoroutineScope()
    var title by remember { mutableStateOf("") }
    var description by remember { mutableStateOf("") }
    var location by remember { mutableStateOf("") }
    var startsAt by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

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
                value = title,
                onValueChange = { title = it },
                label = { Text("Title") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            OutlinedTextField(
                value = description,
                onValueChange = { description = it },
                label = { Text("Description") },
                modifier = Modifier.fillMaxWidth(),
                maxLines = 4,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            OutlinedTextField(
                value = location,
                onValueChange = { location = it },
                label = { Text("Location") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.md))

            OutlinedTextField(
                value = startsAt,
                onValueChange = { startsAt = it },
                label = { Text("Start date (YYYY-MM-DD HH:mm)") },
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            error?.let {
                Spacer(modifier = Modifier.height(PoziomkiTheme.spacing.sm))
                Text(text = it, color = androidx.compose.material3.MaterialTheme.colorScheme.error)
            }

            Spacer(modifier = Modifier.weight(1f))

            Button(
                onClick = {
                    scope.launch {
                        isLoading = true
                        val request =
                            CreateEventRequest(
                                title = title,
                                description = description.ifBlank { null },
                                location = location.ifBlank { null },
                                startsAt = startsAt,
                            )
                        when (val result = apiService.createEvent(request)) {
                            is ApiResult.Success -> onCreated()
                            is ApiResult.Error -> error = result.message
                        }
                        isLoading = false
                    }
                },
                modifier =
                    Modifier
                        .fillMaxWidth()
                        .height(PoziomkiTheme.componentSizes.buttonHeight),
                enabled = !isLoading && title.isNotBlank() && startsAt.isNotBlank(),
            ) {
                if (isLoading) CircularProgressIndicator() else Text("Create Event")
            }
        }
    }
}
